use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tracing::info;
use tracing_subscriber::EnvFilter;
use zeroize::Zeroize;

// ── Configuration ─────────────────────────────────────────────────────────────

/// The Ed25519 key that signs compiled documents, and its did:key.
struct Signer {
    key: alice_sign::KeyPair,
    did: String,
}

impl fmt::Debug for Signer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Signer").field("did", &self.did).finish()
    }
}

struct Config {
    signer: Signer,
    audit_log_id: u64,
}

/// Why the engine refuses to start. No variant carries the seed or any part of it.
#[derive(Debug, PartialEq, Eq)]
enum ConfigError {
    MissingSeed,
    SeedLength,
    SeedNotHex,
    MissingLogId,
    LogIdNotNumber,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingSeed => "LEGAL_SIGNING_SEED is not set",
            Self::SeedLength => "LEGAL_SIGNING_SEED must be 64 hex characters (a 32-byte seed)",
            Self::SeedNotHex => "LEGAL_SIGNING_SEED must be 64 hex characters (a 32-byte seed)",
            Self::MissingLogId => "LEGAL_AUDIT_LOG_ID is not set",
            Self::LogIdNotNumber => "LEGAL_AUDIT_LOG_ID must be an unsigned 64-bit integer",
        })
    }
}

/// The configuration read through `get` (the process environment in `main`).
///
/// `LEGAL_SIGNING_SEED` is the 32-byte RFC 8032 secret as 64 hex characters; its text and its
/// bytes are zeroized once the key is derived, whether or not they were valid.
fn config_from(get: impl Fn(&str) -> Option<String>) -> Result<Config, ConfigError> {
    let mut text = get("LEGAL_SIGNING_SEED").unwrap_or_default();
    let key = seed_key(&text);
    text.zeroize();
    let key = key?;
    let audit_log_id = get("LEGAL_AUDIT_LOG_ID")
        .ok_or(ConfigError::MissingLogId)?
        .trim()
        .parse::<u64>()
        .map_err(|_| ConfigError::LogIdNotNumber)?;
    let did = alice_identity::did::Did::key_ed25519(&key.public()).to_string();
    Ok(Config {
        signer: Signer { key, did },
        audit_log_id,
    })
}

fn seed_key(text: &str) -> Result<alice_sign::KeyPair, ConfigError> {
    let text = text.trim().as_bytes();
    if text.is_empty() {
        return Err(ConfigError::MissingSeed);
    }
    if text.len() != 64 {
        return Err(ConfigError::SeedLength);
    }
    let mut seed = [0u8; 32];
    for (i, pair) in text.chunks_exact(2).enumerate() {
        match (nibble(pair[0]), nibble(pair[1])) {
            (Some(h), Some(l)) => seed[i] = h << 4 | l,
            _ => {
                seed.zeroize();
                return Err(ConfigError::SeedNotHex);
            }
        }
    }
    let key = alice_sign::KeyPair::from_seed(seed);
    seed.zeroize();
    Ok(key)
}

fn nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    start_time: Arc<Instant>,
    signer: Arc<Signer>,
    /// held in memory: it starts empty at each start of the engine
    audit: Arc<Mutex<Audit>>,
}

/// The audit log and, for each entry, the content text its `content_hash` is the identifier of.
struct Audit {
    log: alice_legal::AuditLog,
    contents: Vec<String>,
}

impl AppState {
    fn new(config: Config) -> Self {
        Self {
            start_time: Arc::new(Instant::now()),
            signer: Arc::new(config.signer),
            audit: Arc::new(Mutex::new(Audit {
                log: alice_legal::AuditLog::new(config.audit_log_id),
                contents: Vec::new(),
            })),
        }
    }

    /// Append one entry for an answered request (its entity id is its sequence; its content is
    /// `outcome` as JSON, with sorted keys) and return the head after it.
    fn record(&self, kind: alice_legal::AuditEventKind, outcome: &serde_json::Value) -> String {
        let content = outcome.to_string();
        let mut audit = self.audit.lock().expect("audit log");
        let seq = audit.log.next_sequence;
        audit
            .log
            .append(kind, seq, &self.signer.did, &content, now_ns());
        audit.contents.push(content);
        hex(&audit.log.head())
    }
}

/// The clauses of `document`; a document without a clause is refused, and the refusal is recorded.
fn split_or_record(
    state: &AppState,
    request: &str,
    document: &str,
    document_sha256: &str,
) -> Result<Vec<alice_legal_ai::Clause>, StatusCode> {
    alice_legal_ai::split_clauses(document).map_err(|_| {
        state.record(
            alice_legal::AuditEventKind::DocumentAnalyzed,
            &json!({"request": request, "outcome": "refused", "reason": "no-clauses",
                    "document_sha256": document_sha256}),
        );
        StatusCode::BAD_REQUEST
    })
}

fn now_ns() -> u64 {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970");
    u64::try_from(d.as_nanos()).expect("time fits in u64 nanoseconds")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AnalyzeRequest {
    document: String,
    language: String,
}

#[derive(Debug, Serialize)]
struct Clause {
    id: String,
    section: String,
    text: String,
    clause_type: String,
    risk_level: String,
}

#[derive(Debug, Serialize)]
struct Issue {
    id: String,
    description: String,
    severity: String,
    location: String,
}

#[derive(Debug, Serialize)]
struct AnalyzeResponse {
    risk_score: f64,
    clauses: Vec<Clause>,
    issues: Vec<Issue>,
    language: String,
    word_count: usize,
    /// the audit log head after this analysis was recorded
    audit_head: String,
}

#[derive(Debug, Deserialize)]
struct CompileRequest {
    template_id: String,
    variables: HashMap<String, String>,
    /// the DID of the other party of the compiled document
    counterparty_did: String,
}

/// The contract record the engine signed, with its digest, the engine public key and the
/// Ed25519 signature over the record's canonical bytes.
#[derive(Debug, Serialize)]
struct SignedRecord {
    id: String,
    kind: String,
    issuer_did: String,
    counterparty_did: String,
    effective_unix: u64,
    metadata: BTreeMap<String, String>,
    digest: String,
    public_key: String,
    signature: String,
}

#[derive(Debug, Serialize)]
struct CompileResponse {
    template_id: String,
    compiled_document: String,
    variables_applied: usize,
    missing_variables: Vec<String>,
    /// present only when no variable is missing
    signed: Option<SignedRecord>,
    /// the audit log head after this request was recorded
    audit_head: String,
}

#[derive(Debug, Serialize)]
struct AuditEntryView {
    sequence: u64,
    kind: String,
    entity_id: u64,
    actor_hash: String,
    timestamp_ns: u64,
    /// the request and its outcome, as JSON; `content_hash` is its identifier
    content: String,
    content_hash: String,
    prev_hash: String,
}

#[derive(Debug, Serialize)]
struct AuditResponse {
    log_id: u64,
    head: String,
    entries: Vec<AuditEntryView>,
}

#[derive(Debug, Serialize)]
struct TemplateInfo {
    id: String,
    name: String,
    description: String,
    required_variables: Vec<String>,
    language_support: Vec<String>,
}

#[derive(Debug, Serialize)]
struct TemplatesResponse {
    templates: Vec<TemplateInfo>,
    count: usize,
}

#[derive(Debug, Deserialize)]
struct RiskRequest {
    document: String,
}

#[derive(Debug, Serialize)]
struct RiskFactor {
    factor: String,
    weight: f64,
    score: f64,
    description: String,
}

#[derive(Debug, Serialize)]
struct RiskScoreResponse {
    overall_score: f64,
    risk_level: String,
    risk_factors: Vec<RiskFactor>,
    recommendations: Vec<String>,
    /// the audit log head after this request was recorded
    audit_head: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    uptime_secs: u64,
    service: String,
    version: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_secs: uptime,
        service: "alice-legal-engine".to_string(),
        // health が返す版数は Cargo.toml から取る (literal は bump で drift して監視側に嘘を返す)
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn analyze(
    State(state): State<AppState>,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<AnalyzeResponse>, StatusCode> {
    // every clause and issue comes from the document: an empty one, or one without a clause, is
    // refused rather than answered (and the refusal is recorded)
    let document_sha256 = alice_sign::hash_data(req.document.as_bytes()).to_hex();
    let found = split_or_record(&state, "analyze", &req.document, &document_sha256)?;
    let word_count = req.document.split_whitespace().count();
    let issues = alice_legal_ai::find_issues(&found)
        .into_iter()
        .map(|i| Issue {
            id: i.id,
            description: i.description,
            severity: risk_name(i.severity).to_string(),
            location: i.location,
        })
        .collect();
    let risk_score = alice_legal_ai::contract_risk_score(&found) / 100.0;
    let clauses = found.into_iter().map(clause_of).collect::<Vec<_>>();
    let audit_head = state.record(
        alice_legal::AuditEventKind::DocumentAnalyzed,
        &json!({"request": "analyze", "outcome": "analyzed", "document_sha256": document_sha256}),
    );

    info!(
        language = %req.language,
        word_count,
        clauses = clauses.len(),
        risk_score,
        "document analyzed"
    );

    Ok(Json(AnalyzeResponse {
        risk_score,
        clauses,
        issues,
        language: req.language,
        word_count,
        audit_head,
    }))
}

async fn compile(
    State(state): State<AppState>,
    Json(req): Json<CompileRequest>,
) -> Result<Json<CompileResponse>, StatusCode> {
    let refuse = |reason: &str, status: StatusCode| {
        state.record(
            alice_legal::AuditEventKind::TemplateCompiled,
            &json!({"request": "compile", "outcome": "refused", "reason": reason,
                    "template_id": req.template_id}),
        );
        Err(status)
    };
    if req.template_id.trim().is_empty() {
        return refuse("empty-template-id", StatusCode::BAD_REQUEST);
    }
    if alice_identity::did::Did::parse(&req.counterparty_did).is_err() {
        return refuse("counterparty-not-a-did", StatusCode::BAD_REQUEST);
    }
    let Some(mut compiled) = get_template_body(&req.template_id) else {
        return refuse("unknown-template", StatusCode::NOT_FOUND);
    };
    let mut variables_applied = 0usize;
    let mut missing_variables: Vec<String> = Vec::new();

    // Replace template placeholders with provided variables
    let required = get_required_variables(&req.template_id);
    for var in &required {
        let placeholder = format!("{{{{{}}}}}", var);
        if let Some(value) = req.variables.get(var) {
            compiled = compiled.replace(&placeholder, value);
            variables_applied += 1;
        } else {
            missing_variables.push(var.clone());
        }
    }

    // only a complete document is signed; either way the request is recorded with its outcome
    let (signed, audit_head) = if missing_variables.is_empty() {
        let signed = sign_document(&state, &req, &compiled);
        let head = state.record(
            alice_legal::AuditEventKind::TemplateCompiled,
            &json!({"request": "compile", "outcome": "signed", "template_id": req.template_id,
                    "digest": signed.digest}),
        );
        (Some(signed), head)
    } else {
        let head = state.record(
            alice_legal::AuditEventKind::TemplateCompiled,
            &json!({"request": "compile", "outcome": "incomplete", "template_id": req.template_id,
                    "missing": missing_variables}),
        );
        (None, head)
    };

    info!(
        template_id = %req.template_id,
        variables_applied,
        missing = missing_variables.len(),
        signed = signed.is_some(),
        "template compiled"
    );

    Ok(Json(CompileResponse {
        template_id: req.template_id,
        compiled_document: compiled,
        variables_applied,
        missing_variables,
        signed,
        audit_head,
    }))
}

/// The record of a compiled document (its template and SHA-256 in the metadata), signed with the
/// engine key. Its id is the audit sequence its entry takes.
fn sign_document(state: &AppState, req: &CompileRequest, document: &str) -> SignedRecord {
    let (log_id, seq) = {
        let audit = state.audit.lock().expect("audit log");
        (audit.log.log_id, audit.log.next_sequence)
    };
    let effective_unix = now_ns() / 1_000_000_000;
    let mut record = alice_legal::ContractRecord::new(
        format!("{log_id}-{seq}"),
        "template-compiled",
        state.signer.did.clone(),
        req.counterparty_did.clone(),
        effective_unix,
    );
    record
        .metadata
        .insert("template_id".to_string(), req.template_id.clone());
    record.metadata.insert(
        "document_sha256".to_string(),
        alice_sign::hash_data(document.as_bytes()).to_hex(),
    );
    let mut contract = alice_legal::SignedContract::new(record);
    contract.sign(&state.signer.key);
    let (public_key, signature) = contract.signatures[0];
    let r = contract.record;
    SignedRecord {
        digest: r.digest().to_hex(),
        id: r.id,
        kind: r.kind,
        issuer_did: r.issuer_did,
        counterparty_did: r.counterparty_did,
        effective_unix: r.effective_unix,
        metadata: r.metadata,
        public_key: public_key.to_hex(),
        signature: signature.to_hex(),
    }
}

async fn audit(State(state): State<AppState>) -> Json<AuditResponse> {
    let audit = state.audit.lock().expect("audit log");
    let log = &audit.log;
    let entries = log
        .entries
        .iter()
        .zip(&audit.contents)
        .map(|(e, content)| AuditEntryView {
            sequence: e.sequence,
            kind: e.kind.tag().to_string(),
            entity_id: e.entity_id,
            actor_hash: hex(&e.actor_hash),
            timestamp_ns: e.timestamp_ns,
            content: content.clone(),
            content_hash: hex(&e.content_hash),
            prev_hash: hex(&e.prev_hash),
        })
        .collect();
    Json(AuditResponse {
        log_id: log.log_id,
        head: hex(&log.head()),
        entries,
    })
}

async fn templates(State(_state): State<AppState>) -> Json<TemplatesResponse> {
    let templates = vec![
        TemplateInfo {
            id: "nda".to_string(),
            name: "Non-Disclosure Agreement".to_string(),
            description: "Mutual or one-way NDA for confidential information protection."
                .to_string(),
            required_variables: vec![
                "party_a".to_string(),
                "party_b".to_string(),
                "effective_date".to_string(),
                "jurisdiction".to_string(),
            ],
            language_support: vec!["en".to_string(), "ja".to_string(), "de".to_string()],
        },
        TemplateInfo {
            id: "sla".to_string(),
            name: "Service Level Agreement".to_string(),
            description: "SLA defining uptime guarantees, response times, and remedies."
                .to_string(),
            required_variables: vec![
                "service_provider".to_string(),
                "customer".to_string(),
                "uptime_percent".to_string(),
                "response_time_hours".to_string(),
            ],
            language_support: vec!["en".to_string(), "ja".to_string()],
        },
        TemplateInfo {
            id: "dpa".to_string(),
            name: "Data Processing Agreement".to_string(),
            description: "GDPR-compliant DPA for data controller/processor relationships."
                .to_string(),
            required_variables: vec![
                "controller".to_string(),
                "processor".to_string(),
                "data_types".to_string(),
                "retention_period".to_string(),
            ],
            language_support: vec!["en".to_string(), "de".to_string(), "fr".to_string()],
        },
        TemplateInfo {
            id: "tos".to_string(),
            name: "Terms of Service".to_string(),
            description: "User-facing terms governing use of a product or platform.".to_string(),
            required_variables: vec![
                "company_name".to_string(),
                "product_name".to_string(),
                "governing_law".to_string(),
            ],
            language_support: vec!["en".to_string(), "ja".to_string(), "fr".to_string()],
        },
        TemplateInfo {
            id: "privacy".to_string(),
            name: "Privacy Policy".to_string(),
            description: "GDPR/CCPA-compliant privacy policy for data collection disclosure."
                .to_string(),
            required_variables: vec![
                "company_name".to_string(),
                "contact_email".to_string(),
                "data_collected".to_string(),
            ],
            language_support: vec![
                "en".to_string(),
                "ja".to_string(),
                "de".to_string(),
                "fr".to_string(),
            ],
        },
        TemplateInfo {
            id: "employment".to_string(),
            name: "Employment Agreement".to_string(),
            description:
                "Standard employment contract with salary, IP assignment, and non-compete."
                    .to_string(),
            required_variables: vec![
                "employer".to_string(),
                "employee".to_string(),
                "start_date".to_string(),
                "salary".to_string(),
                "position".to_string(),
            ],
            language_support: vec!["en".to_string(), "ja".to_string()],
        },
        TemplateInfo {
            id: "license".to_string(),
            name: "Software License Agreement".to_string(),
            description: "Commercial software license with usage restrictions and royalties."
                .to_string(),
            required_variables: vec![
                "licensor".to_string(),
                "licensee".to_string(),
                "software_name".to_string(),
                "license_fee".to_string(),
            ],
            language_support: vec!["en".to_string(), "de".to_string()],
        },
    ];

    let count = templates.len();
    Json(TemplatesResponse { templates, count })
}

async fn risk_score(
    State(state): State<AppState>,
    Json(req): Json<RiskRequest>,
) -> Result<Json<RiskScoreResponse>, StatusCode> {
    let document_sha256 = alice_sign::hash_data(req.document.as_bytes()).to_hex();
    let found = split_or_record(&state, "risk-score", &req.document, &document_sha256)?;
    let overall_score = alice_legal_ai::contract_risk_score(&found) / 100.0;

    // one factor per clause type the document has: its score is the highest risk among those
    // clauses, its weight the share of the clauses it holds
    let mut risk_factors: Vec<RiskFactor> = Vec::new();
    for ty in CLAUSE_TYPES {
        let of_type: Vec<&alice_legal_ai::Clause> =
            found.iter().filter(|c| c.clause_type == ty).collect();
        if of_type.is_empty() {
            continue;
        }
        let score = of_type
            .iter()
            .map(|c| risk_value(c.risk_level))
            .fold(0.0_f64, f64::max);
        risk_factors.push(RiskFactor {
            factor: type_name(ty).to_string(),
            weight: of_type.len() as f64 / found.len() as f64,
            score,
            description: format!("{} clause(s) of this type in the document.", of_type.len()),
        });
    }

    let risk_level = match overall_score {
        s if s >= 0.7 => "critical",
        s if s >= 0.5 => "high",
        s if s >= 0.3 => "medium",
        _ => "low",
    }
    .to_string();

    let recommendations = build_recommendations(&risk_level);
    let audit_head = state.record(
        alice_legal::AuditEventKind::DocumentAnalyzed,
        &json!({"request": "risk-score", "outcome": "scored", "document_sha256": document_sha256}),
    );

    info!(
        overall_score,
        risk_level = %risk_level,
        clauses = found.len(),
        "risk score computed"
    );

    Ok(Json(RiskScoreResponse {
        overall_score,
        risk_level,
        risk_factors,
        recommendations,
        audit_head,
    }))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

const CLAUSE_TYPES: [alice_legal_ai::ClauseType; 8] = [
    alice_legal_ai::ClauseType::Indemnification,
    alice_legal_ai::ClauseType::Limitation,
    alice_legal_ai::ClauseType::Termination,
    alice_legal_ai::ClauseType::Confidentiality,
    alice_legal_ai::ClauseType::Ip,
    alice_legal_ai::ClauseType::Warranty,
    alice_legal_ai::ClauseType::Governing,
    alice_legal_ai::ClauseType::Other,
];

fn type_name(ty: alice_legal_ai::ClauseType) -> &'static str {
    use alice_legal_ai::ClauseType as T;
    match ty {
        T::Indemnification => "Indemnification",
        T::Limitation => "Limitation of Liability",
        T::Termination => "Termination",
        T::Confidentiality => "Confidentiality",
        T::Ip => "Intellectual Property",
        T::Warranty => "Warranty",
        T::Governing => "Governing Law",
        T::Other => "Other",
    }
}

fn risk_name(level: alice_legal_ai::RiskLevel) -> &'static str {
    use alice_legal_ai::RiskLevel as R;
    match level {
        R::Low => "low",
        R::Medium => "medium",
        R::High => "high",
        R::Critical => "critical",
    }
}

fn risk_value(level: alice_legal_ai::RiskLevel) -> f64 {
    use alice_legal_ai::RiskLevel as R;
    match level {
        R::Low => 0.1,
        R::Medium => 0.3,
        R::High => 0.7,
        R::Critical => 1.0,
    }
}

fn clause_of(c: alice_legal_ai::Clause) -> Clause {
    Clause {
        id: c.id,
        section: c.section,
        clause_type: type_name(c.clause_type).to_string(),
        risk_level: risk_name(c.risk_level).to_string(),
        text: c.text,
    }
}

fn get_template_body(template_id: &str) -> Option<String> {
    match template_id {
        "nda" => Some(
            "NON-DISCLOSURE AGREEMENT\n\nThis Agreement is entered into between {{party_a}} \
            and {{party_b}}, effective {{effective_date}}, governed by the laws of {{jurisdiction}}.\n\
            \nAll confidential information shared between the parties shall remain strictly \
            confidential for a period of three (3) years.".to_string()
        ),
        "sla" => Some(
            "SERVICE LEVEL AGREEMENT\n\n{{service_provider}} agrees to provide services to \
            {{customer}} with a minimum uptime of {{uptime_percent}}%.\n\
            \nIncident response time shall not exceed {{response_time_hours}} hours.".to_string()
        ),
        "dpa" => Some(
            "DATA PROCESSING AGREEMENT\n\n{{controller}} (Controller) and {{processor}} (Processor) \
            enter into this DPA pursuant to GDPR Article 28.\n\
            \nData types processed: {{data_types}}. Retention period: {{retention_period}}.".to_string()
        ),
        "tos" => Some(
            "TERMS OF SERVICE\n\n{{company_name}} operates {{product_name}}. By using our service, \
            you agree to these terms.\n\
            \nThis agreement is governed by the laws of {{governing_law}}.".to_string()
        ),
        "privacy" => Some(
            "PRIVACY POLICY\n\n{{company_name}} is committed to protecting your privacy. \
            Contact us at {{contact_email}}.\n\
            \nWe collect the following data: {{data_collected}}.".to_string()
        ),
        "employment" => Some(
            "EMPLOYMENT AGREEMENT\n\n{{employer}} employs {{employee}} as {{position}}, \
            commencing {{start_date}}, at an annual salary of {{salary}}.".to_string()
        ),
        "license" => Some(
            "SOFTWARE LICENSE AGREEMENT\n\n{{licensor}} grants {{licensee}} a non-exclusive license \
            to use {{software_name}} subject to payment of {{license_fee}}.".to_string()
        ),
        _ => None,
    }
}

fn get_required_variables(template_id: &str) -> Vec<String> {
    match template_id {
        "nda" => vec!["party_a", "party_b", "effective_date", "jurisdiction"],
        "sla" => vec![
            "service_provider",
            "customer",
            "uptime_percent",
            "response_time_hours",
        ],
        "dpa" => vec!["controller", "processor", "data_types", "retention_period"],
        "tos" => vec!["company_name", "product_name", "governing_law"],
        "privacy" => vec!["company_name", "contact_email", "data_collected"],
        "employment" => vec!["employer", "employee", "start_date", "salary", "position"],
        "license" => vec!["licensor", "licensee", "software_name", "license_fee"],
        _ => vec![],
    }
    .into_iter()
    .map(String::from)
    .collect()
}

fn build_recommendations(risk_level: &str) -> Vec<String> {
    match risk_level {
        "critical" => vec![
            "Engage qualified legal counsel before signing.".to_string(),
            "Negotiate liability cap to a fixed monetary amount.".to_string(),
            "Request mutual indemnification rather than one-sided obligation.".to_string(),
            "Add dispute resolution and arbitration clause.".to_string(),
        ],
        "high" => vec![
            "Review indemnification scope with an attorney.".to_string(),
            "Clarify IP ownership provisions.".to_string(),
            "Ensure termination notice periods are reasonable.".to_string(),
        ],
        "medium" => vec![
            "Verify jurisdiction and governing law aligns with your location.".to_string(),
            "Confirm data retention periods meet regulatory requirements.".to_string(),
        ],
        _ => vec!["Document appears low risk. Standard review recommended.".to_string()],
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    // the configuration is read before any thread starts; without it the engine does not start
    let config = match config_from(|k| std::env::var(k).ok()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("legal-engine: {e}");
            std::process::exit(2);
        }
    };
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(serve(config));
}

async fn serve(config: Config) {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("legal_engine=info,tower_http=debug")),
        )
        .init();

    info!(issuer_did = %config.signer.did, audit_log_id = config.audit_log_id, "signing key loaded");
    let state = AppState::new(config);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/legal/analyze", post(analyze))
        .route("/api/v1/legal/compile", post(compile))
        .route("/api/v1/legal/templates", get(templates))
        .route("/api/v1/legal/risk-score", post(risk_score))
        .route("/api/v1/legal/audit", get(audit))
        .with_state(state);

    let addr_str = std::env::var("LEGAL_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".to_string());
    let addr: SocketAddr = addr_str.parse().expect("invalid LEGAL_ADDR");

    info!("ALICE Legal Engine listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// health が名乗る版数は Cargo.toml の version と常に一致する
    /// (literal に戻すと監視 / deploy 側が古い版数を掴む)
    #[tokio::test]
    async fn health_version_matches_cargo_pkg_version() {
        let Json(body) = health(state()).await;
        assert_eq!(body.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(body.status, "ok");
    }

    /// a seed made inside the test (no key or seed is kept in a fixture)
    fn test_seed(salt: u8) -> String {
        (0u8..32)
            .map(|i| format!("{:02x}", i.wrapping_mul(37).wrapping_add(salt)))
            .collect()
    }

    fn env_of(pairs: Vec<(&'static str, String)>) -> impl Fn(&str) -> Option<String> {
        move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone())
    }

    fn config(salt: u8) -> Config {
        config_from(env_of(vec![
            ("LEGAL_SIGNING_SEED", test_seed(salt)),
            ("LEGAL_AUDIT_LOG_ID", "7".to_string()),
        ]))
        .expect("config")
    }

    fn state() -> State<AppState> {
        State(AppState::new(config(1)))
    }

    const NO_INDEMNIFICATION: &str =
        "1. Confidentiality. Each party keeps the other's information confidential.\n\
        2. Termination. Either party may terminate on material breach.\n\
        3. Governing law. This Agreement is governed by the laws of Japan.\n";

    fn collapsed(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// every clause and issue comes from the document: nothing is reported that it does not hold
    #[tokio::test]
    async fn analyze_reports_only_what_the_document_holds() {
        let req = AnalyzeRequest {
            document: NO_INDEMNIFICATION.to_string(),
            language: "en".to_string(),
        };
        let Json(body) = analyze(state(), Json(req)).await.expect("analyzed");
        let expected = alice_legal_ai::split_clauses(NO_INDEMNIFICATION).expect("clauses");
        assert_eq!(body.clauses.len(), expected.len());
        let doc = collapsed(NO_INDEMNIFICATION);
        let sections: Vec<&str> = body.clauses.iter().map(|c| c.section.as_str()).collect();
        assert_eq!(sections, ["1", "2", "3"]);
        for c in &body.clauses {
            assert!(doc.contains(&c.text), "{c:?} is not text of the document");
        }
        for i in &body.issues {
            match i.location.strip_prefix("Section ") {
                Some(section) => assert!(sections.contains(&section), "{i:?}"),
                None => {
                    assert_eq!(i.location, "document", "{i:?}");
                    assert!(
                        i.description.starts_with("No ")
                            && i.description.ends_with(" clause found."),
                        "{i:?}"
                    );
                }
            }
        }
        // the document has no indemnification clause: the only indemnification issue is that it is missing
        let about: Vec<&Issue> = body
            .issues
            .iter()
            .filter(|i| i.description.to_lowercase().contains("indemnif"))
            .collect();
        assert_eq!(about.len(), 1, "{about:?}");
        assert_eq!(about[0].description, "No Indemnification clause found.");
        assert_eq!(
            body.risk_score,
            alice_legal_ai::contract_risk_score(&expected) / 100.0
        );
    }

    #[tokio::test]
    async fn analyze_refuses_a_document_without_clauses() {
        for document in ["", "   \n", "1.\n2.\n"] {
            let req = AnalyzeRequest {
                document: document.to_string(),
                language: "en".to_string(),
            };
            assert_eq!(
                analyze(state(), Json(req)).await.err(),
                Some(StatusCode::BAD_REQUEST),
                "{document:?}"
            );
        }
    }

    /// the score and the factors come from the clauses: a type the document lacks has no factor
    #[tokio::test]
    async fn risk_score_is_computed_from_the_clauses() {
        let req = RiskRequest {
            document: NO_INDEMNIFICATION.to_string(),
        };
        let Json(body) = risk_score(state(), Json(req)).await.expect("scored");
        let expected = alice_legal_ai::split_clauses(NO_INDEMNIFICATION).expect("clauses");
        assert_eq!(
            body.overall_score,
            alice_legal_ai::contract_risk_score(&expected) / 100.0
        );
        let factors: Vec<&str> = body
            .risk_factors
            .iter()
            .map(|f| f.factor.as_str())
            .collect();
        assert_eq!(factors, ["Termination", "Confidentiality", "Governing Law"]);
        let total: f64 = body.risk_factors.iter().map(|f| f.weight).sum();
        assert!((total - 1.0).abs() < 1e-12, "{total}");
        let req = RiskRequest {
            document: String::new(),
        };
        assert_eq!(
            risk_score(state(), Json(req)).await.err(),
            Some(StatusCode::BAD_REQUEST)
        );
    }

    // ── signing key and audit log configuration ──────────────────────────────

    /// the engine does not start without its signing seed and its audit log id, and no error
    /// carries the seed text
    #[test]
    fn the_configuration_is_refused_without_a_seed_or_a_log_id() {
        let seed = test_seed(3);
        let log = || ("LEGAL_AUDIT_LOG_ID", "7".to_string());
        let cases: Vec<(Vec<(&'static str, String)>, ConfigError)> = vec![
            (vec![log()], ConfigError::MissingSeed),
            (
                vec![("LEGAL_SIGNING_SEED", String::new()), log()],
                ConfigError::MissingSeed,
            ),
            (
                vec![("LEGAL_SIGNING_SEED", seed[..62].to_string()), log()],
                ConfigError::SeedLength,
            ),
            (
                vec![("LEGAL_SIGNING_SEED", format!("{seed}00")), log()],
                ConfigError::SeedLength,
            ),
            (
                vec![("LEGAL_SIGNING_SEED", format!("{}zz", &seed[..62])), log()],
                ConfigError::SeedNotHex,
            ),
            (
                vec![("LEGAL_SIGNING_SEED", seed.clone())],
                ConfigError::MissingLogId,
            ),
            (
                vec![
                    ("LEGAL_SIGNING_SEED", seed.clone()),
                    ("LEGAL_AUDIT_LOG_ID", "seven".to_string()),
                ],
                ConfigError::LogIdNotNumber,
            ),
        ];
        for (pairs, want) in cases {
            let shown = format!("{pairs:?}");
            let got = config_from(env_of(pairs)).err();
            assert_eq!(got, Some(want), "{shown}");
            let text = got.unwrap().to_string();
            assert!(!text.contains(&seed[..16]), "{text}");
        }
    }

    /// the key is the one the seed names, and the issuer DID is its did:key
    #[test]
    fn the_issuer_did_is_the_did_key_of_the_seed_key() {
        let c = config(5);
        let seed: [u8; 32] = core::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(5));
        let pk = alice_sign::KeyPair::from_seed(seed).public();
        assert_eq!(c.signer.key.public(), pk);
        assert_eq!(
            c.signer.did,
            alice_identity::did::Did::key_ed25519(&pk).to_string()
        );
        assert_eq!(c.audit_log_id, 7);
        // the key is not shown in debug output
        assert!(!format!("{:?}", c.signer).contains(&test_seed(5)[..16]));
    }

    // ── compile signs the document and records it ────────────────────────────

    fn nda(counterparty: &str) -> CompileRequest {
        let variables = [
            ("party_a", "Alpha KK"),
            ("party_b", "Beta GmbH"),
            ("effective_date", "2026-10-25"),
            ("jurisdiction", "Japan"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        CompileRequest {
            template_id: "nda".to_string(),
            variables,
            counterparty_did: counterparty.to_string(),
        }
    }

    fn other_party() -> String {
        let pk = alice_sign::KeyPair::from_seed([9u8; 32]).public();
        alice_identity::did::Did::key_ed25519(&pk).to_string()
    }

    fn record_of(s: &SignedRecord) -> alice_legal::ContractRecord {
        let mut r = alice_legal::ContractRecord::new(
            s.id.clone(),
            s.kind.clone(),
            s.issuer_did.clone(),
            s.counterparty_did.clone(),
            s.effective_unix,
        );
        r.metadata = s.metadata.clone();
        r
    }

    /// the compiled document is signed by the engine key: the signature verifies over the record,
    /// the record carries the document's SHA-256, and the issuer DID gives back the signing key
    #[tokio::test]
    async fn a_complete_compile_is_signed_over_the_document() {
        let st = state();
        let key = st.0.signer.key.public();
        let Json(body) = compile(st, Json(nda(&other_party())))
            .await
            .expect("compiled");
        assert!(body.missing_variables.is_empty());
        let signed = body.signed.expect("a complete document is signed");
        assert_eq!(signed.counterparty_did, other_party());
        assert_eq!(signed.kind, "template-compiled");
        assert_eq!(
            signed.metadata.get("template_id").map(String::as_str),
            Some("nda")
        );
        assert_eq!(
            signed.metadata.get("document_sha256"),
            Some(&alice_sign::hash_data(body.compiled_document.as_bytes()).to_hex())
        );
        let record = record_of(&signed);
        assert_eq!(signed.digest, record.digest().to_hex());
        let pk = alice_sign::PublicKey::from_hex(&signed.public_key).expect("public key");
        assert_eq!(pk, key);
        let sig = alice_sign::Signature::from_hex(&signed.signature).expect("signature");
        assert!(pk.verify_strict(&record.canonical_bytes(), &sig));
        let issuer = alice_identity::did::Did::parse(&signed.issuer_did).expect("a DID");
        assert_eq!(issuer.ed25519_public_key().expect("did:key"), pk);
        // a changed document no longer matches what was signed
        let mut changed = record.clone();
        changed.metadata.insert(
            "document_sha256".to_string(),
            alice_sign::hash_data(b"another document").to_hex(),
        );
        assert!(!pk.verify_strict(&changed.canonical_bytes(), &sig));
    }

    /// the one entry of a fresh engine after `request`, as its kind and its content (JSON)
    fn only_entry(log: &AuditResponse) -> (&str, serde_json::Value) {
        assert_eq!(log.entries.len(), 1, "{:?}", log.entries);
        let e = &log.entries[0];
        (
            e.kind.as_str(),
            serde_json::from_str(&e.content).expect("JSON content"),
        )
    }

    /// an incomplete document is not signed, but the request is recorded with its outcome
    #[tokio::test]
    async fn an_incomplete_compile_is_recorded_as_incomplete() {
        let st = state();
        let mut req = nda(&other_party());
        req.variables.remove("jurisdiction");
        let Json(body) = compile(st.clone(), Json(req)).await.expect("compiled");
        assert_eq!(body.missing_variables, ["jurisdiction"]);
        assert!(body.signed.is_none());
        let Json(log) = audit(st).await;
        assert_eq!(body.audit_head, log.head);
        let (kind, content) = only_entry(&log);
        assert_eq!(kind, "template-compiled");
        assert_eq!(
            content,
            json!({"request": "compile", "outcome": "incomplete", "template_id": "nda",
                   "missing": ["jurisdiction"]})
        );
    }

    /// every refused compile is recorded with the reason it was refused
    #[tokio::test]
    async fn a_refused_compile_is_recorded_with_its_reason() {
        let mut cases: Vec<(CompileRequest, StatusCode, &str)> = ["", "alice", "did:", "did:key"]
            .into_iter()
            .map(|bad| (nda(bad), StatusCode::BAD_REQUEST, "counterparty-not-a-did"))
            .collect();
        let mut empty = nda(&other_party());
        empty.template_id = " ".to_string();
        cases.push((empty, StatusCode::BAD_REQUEST, "empty-template-id"));
        let mut unknown = nda(&other_party());
        unknown.template_id = "lease".to_string();
        cases.push((unknown, StatusCode::NOT_FOUND, "unknown-template"));
        for (req, status, reason) in cases {
            let st = state();
            let template_id = req.template_id.clone();
            let shown = format!("{:?} {:?}", req.template_id, req.counterparty_did);
            assert_eq!(
                compile(st.clone(), Json(req)).await.err(),
                Some(status),
                "{shown}"
            );
            let Json(log) = audit(st).await;
            let (kind, content) = only_entry(&log);
            assert_eq!(kind, "template-compiled", "{shown}");
            assert_eq!(
                content,
                json!({"request": "compile", "outcome": "refused", "reason": reason,
                       "template_id": template_id}),
                "{shown}"
            );
        }
    }

    /// a document without a clause is refused by analyze and by risk-score, and each refusal is
    /// recorded
    #[tokio::test]
    async fn a_refused_analysis_is_recorded_as_refused() {
        let empty = alice_sign::hash_data(b"").to_hex();
        let st = state();
        let req = AnalyzeRequest {
            document: String::new(),
            language: "en".to_string(),
        };
        assert!(analyze(st.clone(), Json(req)).await.is_err());
        let Json(log) = audit(st).await;
        assert_eq!(
            only_entry(&log),
            (
                "document-analyzed",
                json!({"request": "analyze", "outcome": "refused", "reason": "no-clauses",
                       "document_sha256": empty})
            )
        );
        let st = state();
        let req = RiskRequest {
            document: String::new(),
        };
        assert!(risk_score(st.clone(), Json(req)).await.is_err());
        let Json(log) = audit(st).await;
        assert_eq!(
            only_entry(&log),
            (
                "document-analyzed",
                json!({"request": "risk-score", "outcome": "refused", "reason": "no-clauses",
                       "document_sha256": empty})
            )
        );
    }

    /// each answered analyze, risk-score and compile is one entry; every answer carries the head
    /// after its entry, the log verifies against the last head returned, and each entry's content
    /// is the text its content hash identifies
    #[tokio::test]
    async fn the_audit_log_records_each_answered_request() {
        let st = state();
        let doc = alice_sign::hash_data(NO_INDEMNIFICATION.as_bytes()).to_hex();
        let Json(a) = analyze(
            st.clone(),
            Json(AnalyzeRequest {
                document: NO_INDEMNIFICATION.to_string(),
                language: "en".to_string(),
            }),
        )
        .await
        .expect("analyzed");
        let Json(r) = risk_score(
            st.clone(),
            Json(RiskRequest {
                document: NO_INDEMNIFICATION.to_string(),
            }),
        )
        .await
        .expect("scored");
        let Json(c) = compile(st.clone(), Json(nda(&other_party())))
            .await
            .expect("compiled");
        let Json(log) = audit(st.clone()).await;
        assert_eq!(log.log_id, 7);
        assert_eq!(log.head, c.audit_head);
        let kinds: Vec<&str> = log.entries.iter().map(|e| e.kind.as_str()).collect();
        assert_eq!(
            kinds,
            [
                "document-analyzed",
                "document-analyzed",
                "template-compiled"
            ]
        );
        let seqs: Vec<u64> = log.entries.iter().map(|e| e.sequence).collect();
        assert_eq!(seqs, [0, 1, 2]);
        assert_eq!(log.entries[1].prev_hash, a.audit_head);
        assert_eq!(log.entries[2].prev_hash, r.audit_head);
        let contents: Vec<serde_json::Value> = log
            .entries
            .iter()
            .map(|e| serde_json::from_str(&e.content).expect("JSON"))
            .collect();
        assert_eq!(
            contents,
            [
                json!({"request": "analyze", "outcome": "analyzed", "document_sha256": doc}),
                json!({"request": "risk-score", "outcome": "scored", "document_sha256": doc}),
                json!({"request": "compile", "outcome": "signed", "template_id": "nda",
                       "digest": c.signed.as_ref().expect("signed").digest}),
            ]
        );
        for e in &log.entries {
            assert_eq!(
                e.content_hash,
                hex(&alice_legal::law_id::text(
                    "alice-legal/audit-entry/content/v1",
                    &e.content
                )),
                "{e:?}"
            );
            assert_eq!(
                e.actor_hash,
                hex(&alice_legal::law_id::text(
                    "alice-legal/audit-entry/actor/v1",
                    &st.0.signer.did
                ))
            );
        }
        let inner = st.0.audit.lock().expect("log");
        assert!(inner.log.verify_chain(&unhex32(&c.audit_head)));
        assert!(!inner.log.verify_chain(&unhex32(&r.audit_head)));
        assert!(!inner.log.verify_chain(&unhex32(&a.audit_head)));
    }

    fn unhex32(s: &str) -> [u8; 32] {
        let v: Vec<u8> = (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("hex"))
            .collect();
        v.try_into().expect("32 bytes")
    }
}

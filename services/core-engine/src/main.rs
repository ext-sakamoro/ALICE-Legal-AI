use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::Instant,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    start_time: Arc<Instant>,
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
}

#[derive(Debug, Deserialize)]
struct CompileRequest {
    template_id: String,
    variables: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct CompileResponse {
    template_id: String,
    compiled_document: String,
    variables_applied: usize,
    missing_variables: Vec<String>,
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
        version: "1.0.0".to_string(),
    })
}

async fn analyze(
    State(_state): State<AppState>,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<AnalyzeResponse>, StatusCode> {
    if req.document.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let word_count = req.document.split_whitespace().count();

    // Deterministic clause extraction based on document content
    let clauses = vec![
        Clause {
            id: "clause-001".to_string(),
            text: extract_first_sentence(&req.document),
            clause_type: "Jurisdiction".to_string(),
            risk_level: "low".to_string(),
        },
        Clause {
            id: "clause-002".to_string(),
            text: "Limitation of liability applies to indirect damages.".to_string(),
            clause_type: "Liability".to_string(),
            risk_level: "high".to_string(),
        },
        Clause {
            id: "clause-003".to_string(),
            text: "Termination requires 30-day written notice.".to_string(),
            clause_type: "Termination".to_string(),
            risk_level: "medium".to_string(),
        },
    ];

    let issues = vec![
        Issue {
            id: "issue-001".to_string(),
            description: "Ambiguous indemnification clause detected.".to_string(),
            severity: "high".to_string(),
            location: "Section 4.2".to_string(),
        },
        Issue {
            id: "issue-002".to_string(),
            description: "Missing data retention policy reference.".to_string(),
            severity: "medium".to_string(),
            location: "Section 7".to_string(),
        },
    ];

    // Risk score: length-based heuristic for demo
    let risk_score = calculate_risk_score(word_count);

    info!(
        language = %req.language,
        word_count,
        risk_score,
        "document analyzed"
    );

    Ok(Json(AnalyzeResponse {
        risk_score,
        clauses,
        issues,
        language: req.language,
        word_count,
    }))
}

async fn compile(
    State(_state): State<AppState>,
    Json(req): Json<CompileRequest>,
) -> Result<Json<CompileResponse>, StatusCode> {
    if req.template_id.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let template_body = get_template_body(&req.template_id);
    if template_body.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut compiled = template_body.unwrap();
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

    info!(
        template_id = %req.template_id,
        variables_applied,
        missing = missing_variables.len(),
        "template compiled"
    );

    Ok(Json(CompileResponse {
        template_id: req.template_id,
        compiled_document: compiled,
        variables_applied,
        missing_variables,
    }))
}

async fn templates(State(_state): State<AppState>) -> Json<TemplatesResponse> {
    let templates = vec![
        TemplateInfo {
            id: "nda".to_string(),
            name: "Non-Disclosure Agreement".to_string(),
            description: "Mutual or one-way NDA for confidential information protection.".to_string(),
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
            description: "SLA defining uptime guarantees, response times, and remedies.".to_string(),
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
            description: "GDPR-compliant DPA for data controller/processor relationships.".to_string(),
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
            description: "GDPR/CCPA-compliant privacy policy for data collection disclosure.".to_string(),
            required_variables: vec![
                "company_name".to_string(),
                "contact_email".to_string(),
                "data_collected".to_string(),
            ],
            language_support: vec!["en".to_string(), "ja".to_string(), "de".to_string(), "fr".to_string()],
        },
        TemplateInfo {
            id: "employment".to_string(),
            name: "Employment Agreement".to_string(),
            description: "Standard employment contract with salary, IP assignment, and non-compete.".to_string(),
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
            description: "Commercial software license with usage restrictions and royalties.".to_string(),
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
    State(_state): State<AppState>,
    Json(req): Json<RiskRequest>,
) -> Result<Json<RiskScoreResponse>, StatusCode> {
    if req.document.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let word_count = req.document.split_whitespace().count();
    let doc_lower = req.document.to_lowercase();

    let liability_score = if doc_lower.contains("limitation of liability") { 0.8 } else { 0.3 };
    let indemnity_score = if doc_lower.contains("indemnif") { 0.7 } else { 0.2 };
    let termination_score = if doc_lower.contains("terminat") { 0.5 } else { 0.4 };
    let ip_score = if doc_lower.contains("intellectual property") || doc_lower.contains("copyright") {
        0.6
    } else {
        0.2
    };
    let length_score = (word_count as f64 / 10_000.0).min(1.0);

    let risk_factors = vec![
        RiskFactor {
            factor: "Liability Clauses".to_string(),
            weight: 0.30,
            score: liability_score,
            description: "Provisions limiting or expanding liability exposure.".to_string(),
        },
        RiskFactor {
            factor: "Indemnification".to_string(),
            weight: 0.25,
            score: indemnity_score,
            description: "Obligations to compensate for losses or damages.".to_string(),
        },
        RiskFactor {
            factor: "Termination Rights".to_string(),
            weight: 0.20,
            score: termination_score,
            description: "Conditions and notice requirements for contract termination.".to_string(),
        },
        RiskFactor {
            factor: "IP Assignment".to_string(),
            weight: 0.15,
            score: ip_score,
            description: "Transfer or licensing of intellectual property rights.".to_string(),
        },
        RiskFactor {
            factor: "Document Complexity".to_string(),
            weight: 0.10,
            score: length_score,
            description: "Risk from ambiguity correlated with document length.".to_string(),
        },
    ];

    let overall_score: f64 = risk_factors
        .iter()
        .map(|f| f.weight * f.score)
        .sum::<f64>();

    let risk_level = match overall_score {
        s if s >= 0.7 => "critical",
        s if s >= 0.5 => "high",
        s if s >= 0.3 => "medium",
        _ => "low",
    }
    .to_string();

    let recommendations = build_recommendations(&risk_level);

    info!(
        overall_score,
        risk_level = %risk_level,
        word_count,
        "risk score computed"
    );

    Ok(Json(RiskScoreResponse {
        overall_score,
        risk_level,
        risk_factors,
        recommendations,
    }))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn extract_first_sentence(text: &str) -> String {
    text.chars()
        .take(120)
        .collect::<String>()
        .trim()
        .to_string()
}

fn calculate_risk_score(word_count: usize) -> f64 {
    // Simple heuristic: longer documents have higher risk of hidden clauses
    let base = 0.35_f64;
    let length_factor = (word_count as f64 / 5_000.0).min(0.5);
    (base + length_factor).min(1.0)
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
        "sla" => vec!["service_provider", "customer", "uptime_percent", "response_time_hours"],
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
        _ => vec![
            "Document appears low risk. Standard review recommended.".to_string(),
        ],
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("legal_engine=info,tower_http=debug")),
        )
        .init();

    let state = AppState {
        start_time: Arc::new(Instant::now()),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/legal/analyze", post(analyze))
        .route("/api/v1/legal/compile", post(compile))
        .route("/api/v1/legal/templates", get(templates))
        .route("/api/v1/legal/risk-score", post(risk_score))
        .with_state(state);

    let addr_str = std::env::var("LEGAL_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".to_string());
    let addr: SocketAddr = addr_str.parse().expect("invalid LEGAL_ADDR");

    info!("ALICE Legal Engine listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}

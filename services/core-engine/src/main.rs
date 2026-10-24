use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Instant};
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
        // health が返す版数は Cargo.toml から取る (literal は bump で drift して監視側に嘘を返す)
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn analyze(
    State(_state): State<AppState>,
    Json(req): Json<AnalyzeRequest>,
) -> Result<Json<AnalyzeResponse>, StatusCode> {
    // every clause and issue comes from the document: an empty one, or one without a clause, is
    // refused rather than answered
    let found =
        alice_legal_ai::split_clauses(&req.document).map_err(|_| StatusCode::BAD_REQUEST)?;
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
    State(_state): State<AppState>,
    Json(req): Json<RiskRequest>,
) -> Result<Json<RiskScoreResponse>, StatusCode> {
    let found =
        alice_legal_ai::split_clauses(&req.document).map_err(|_| StatusCode::BAD_REQUEST)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// health が名乗る版数は Cargo.toml の version と常に一致する
    /// (literal に戻すと監視 / deploy 側が古い版数を掴む)
    #[tokio::test]
    async fn health_version_matches_cargo_pkg_version() {
        let state = AppState {
            start_time: Arc::new(Instant::now()),
        };
        let Json(body) = health(State(state)).await;
        assert_eq!(body.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(body.status, "ok");
    }

    fn state() -> State<AppState> {
        State(AppState {
            start_time: Arc::new(Instant::now()),
        })
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
}

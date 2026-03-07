//! ALICE-Legal-AI — Legal document analysis engine
//!
//! 条項抽出、リスクスコアリング、TF-IDF類似度、契約条件チェック

#![no_std]
extern crate alloc;
use alloc::{collections::BTreeMap, string::String, vec::Vec};

// ---------------------------------------------------------------------------
// Document & Clause
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clause {
    pub id: String,
    pub section: String,
    pub text: String,
    pub clause_type: ClauseType,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseType {
    Indemnification,
    Limitation,
    Termination,
    Confidentiality,
    Ip,
    Warranty,
    Governing,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

// ---------------------------------------------------------------------------
// Keyword-based Clause Classification
// ---------------------------------------------------------------------------

/// キーワードに基づく条項分類
#[must_use]
pub fn classify_clause(text: &str) -> ClauseType {
    let lower: Vec<u8> = text
        .bytes()
        .map(|b: u8| if b.is_ascii_uppercase() { b + 32 } else { b })
        .collect();
    let s = core::str::from_utf8(&lower).unwrap_or("");

    if s.contains("indemnif") || s.contains("hold harmless") {
        ClauseType::Indemnification
    } else if s.contains("limitation") || s.contains("liable") || s.contains("damages") {
        ClauseType::Limitation
    } else if s.contains("terminat") || s.contains("cancel") {
        ClauseType::Termination
    } else if s.contains("confidential") || s.contains("non-disclosure") || s.contains("nda") {
        ClauseType::Confidentiality
    } else if s.contains("intellectual property") || s.contains("patent") || s.contains("copyright")
    {
        ClauseType::Ip
    } else if s.contains("warrant") || s.contains("guarantee") {
        ClauseType::Warranty
    } else if s.contains("governing law") || s.contains("jurisdiction") {
        ClauseType::Governing
    } else {
        ClauseType::Other
    }
}

/// 条項のリスクレベル推定
#[must_use]
pub fn assess_risk(text: &str) -> RiskLevel {
    let lower: Vec<u8> = text
        .bytes()
        .map(|b: u8| if b.is_ascii_uppercase() { b + 32 } else { b })
        .collect();
    let s = core::str::from_utf8(&lower).unwrap_or("");

    let high_risk = [
        "unlimited liability",
        "sole discretion",
        "without notice",
        "irrevocable",
        "perpetual",
    ];
    let medium_risk = ["may terminate", "reasonable efforts", "material breach"];

    for &keyword in &high_risk {
        if s.contains(keyword) {
            return RiskLevel::Critical;
        }
    }
    for &keyword in &medium_risk {
        if s.contains(keyword) {
            return RiskLevel::Medium;
        }
    }
    RiskLevel::Low
}

// ---------------------------------------------------------------------------
// TF-IDF Similarity
// ---------------------------------------------------------------------------

/// 単語分割 (簡易: スペース区切り、小文字化)
#[must_use]
pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            current.push(if c.is_uppercase() {
                c.to_ascii_lowercase()
            } else {
                c
            });
        } else if !current.is_empty() {
            tokens.push(core::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// TF: 単語頻度
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn term_frequency(tokens: &[String]) -> BTreeMap<String, f64> {
    let mut tf = BTreeMap::new();
    let n = tokens.len() as f64;
    if n == 0.0 {
        return tf;
    }
    for token in tokens {
        *tf.entry(token.clone()).or_insert(0.0) += 1.0;
    }
    for val in tf.values_mut() {
        *val /= n;
    }
    tf
}

/// コサイン類似度
#[must_use]
pub fn cosine_similarity(tf1: &BTreeMap<String, f64>, tf2: &BTreeMap<String, f64>) -> f64 {
    let mut dot = 0.0;
    let mut norm1 = 0.0;
    let mut norm2 = 0.0;

    for (key, &v1) in tf1 {
        norm1 += v1 * v1;
        if let Some(&v2) = tf2.get(key) {
            dot += v1 * v2;
        }
    }
    for &v2 in tf2.values() {
        norm2 += v2 * v2;
    }

    let denom = sqrt_approx(norm1) * sqrt_approx(norm2);
    if denom < 1e-10 {
        0.0
    } else {
        dot / denom
    }
}

fn sqrt_approx(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut g = x / 2.0;
    for _ in 0..20 {
        g = f64::midpoint(g, x / g);
    }
    g
}

// ---------------------------------------------------------------------------
// Contract Checklist
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecklistItem {
    pub name: String,
    pub required: bool,
    pub found: bool,
}

/// 契約条件チェックリスト生成
#[must_use]
pub fn check_contract(clauses: &[Clause]) -> Vec<ChecklistItem> {
    let required_types = [
        (ClauseType::Indemnification, "Indemnification"),
        (ClauseType::Limitation, "Limitation of Liability"),
        (ClauseType::Termination, "Termination"),
        (ClauseType::Confidentiality, "Confidentiality"),
        (ClauseType::Governing, "Governing Law"),
    ];

    required_types
        .iter()
        .map(|(ct, name)| ChecklistItem {
            name: String::from(*name),
            required: true,
            found: clauses.iter().any(|c| c.clause_type == *ct),
        })
        .collect()
}

/// 契約のリスクスコア: critical条項の割合
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn contract_risk_score(clauses: &[Clause]) -> f64 {
    if clauses.is_empty() {
        return 0.0;
    }
    let risk_sum: u32 = clauses
        .iter()
        .map(|c| match c.risk_level {
            RiskLevel::Low => 1,
            RiskLevel::Medium => 3,
            RiskLevel::High => 7,
            RiskLevel::Critical => 10,
        })
        .sum();
    f64::from(risk_sum) / (clauses.len() as f64 * 10.0) * 100.0
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegalError {
    ParseError,
    ClassificationFailed,
}

impl core::fmt::Display for LegalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ParseError => write!(f, "parse error"),
            Self::ClassificationFailed => write!(f, "classification failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn classify_indemnification() {
        assert_eq!(
            classify_clause("The Vendor shall indemnify the Client"),
            ClauseType::Indemnification
        );
    }

    #[test]
    fn classify_termination() {
        assert_eq!(
            classify_clause("Either party may terminate this agreement"),
            ClauseType::Termination
        );
    }

    #[test]
    fn classify_confidentiality() {
        assert_eq!(
            classify_clause("All confidential information shall be protected"),
            ClauseType::Confidentiality
        );
    }

    #[test]
    fn classify_ip() {
        assert_eq!(
            classify_clause("Intellectual Property rights remain with the author"),
            ClauseType::Ip
        );
    }

    #[test]
    fn risk_critical() {
        assert_eq!(
            assess_risk("The company has unlimited liability for damages"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn risk_medium() {
        assert_eq!(
            assess_risk("Either party may terminate upon material breach"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn risk_low() {
        assert_eq!(assess_risk("Standard terms apply"), RiskLevel::Low);
    }

    #[test]
    fn tokenize_basic() {
        let tokens = tokenize("Hello, World! Test 123");
        assert_eq!(tokens, vec!["hello", "world", "test", "123"]);
    }

    #[test]
    fn cosine_identical() {
        let tf = term_frequency(&tokenize("the quick brown fox"));
        let sim = cosine_similarity(&tf, &tf);
        assert!((sim - 1.0).abs() < 0.01);
    }

    #[test]
    fn cosine_different() {
        let tf1 = term_frequency(&tokenize("legal contract agreement"));
        let tf2 = term_frequency(&tokenize("physics quantum mechanics"));
        let sim = cosine_similarity(&tf1, &tf2);
        assert!(sim < 0.1);
    }

    #[test]
    fn contract_checklist() {
        let clauses = vec![
            Clause {
                id: String::from("1"),
                section: String::from("3"),
                text: String::from("indemnification"),
                clause_type: ClauseType::Indemnification,
                risk_level: RiskLevel::Low,
            },
            Clause {
                id: String::from("2"),
                section: String::from("5"),
                text: String::from("termination"),
                clause_type: ClauseType::Termination,
                risk_level: RiskLevel::Low,
            },
        ];
        let checklist = check_contract(&clauses);
        assert_eq!(checklist.len(), 5);
        assert!(checklist[0].found); // indemnification
        assert!(!checklist[3].found); // confidentiality missing
    }

    #[test]
    fn risk_score_all_low() {
        let clauses = vec![Clause {
            id: String::from("1"),
            section: String::from("1"),
            text: String::new(),
            clause_type: ClauseType::Other,
            risk_level: RiskLevel::Low,
        }];
        let score = contract_risk_score(&clauses);
        assert!((score - 10.0).abs() < 0.01);
    }

    #[test]
    fn risk_score_all_critical() {
        let clauses = vec![Clause {
            id: String::from("1"),
            section: String::from("1"),
            text: String::new(),
            clause_type: ClauseType::Other,
            risk_level: RiskLevel::Critical,
        }];
        let score = contract_risk_score(&clauses);
        assert!((score - 100.0).abs() < 0.01);
    }
}

//! classifier.

use crate::document::*;
use alloc::vec::Vec;

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

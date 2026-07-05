//! checklist.

use crate::document::*;
use alloc::string::String;
use alloc::vec::Vec;

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

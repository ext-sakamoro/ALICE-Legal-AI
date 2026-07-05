//! document.

use alloc::string::String;

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

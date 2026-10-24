//! issues: what a contract's clauses leave out or put at risk.
//!
//! An issue is either a required clause type the document has no clause of ([`check_contract`]),
//! located at `document`, or a clause whose wording is of high or critical risk ([`assess_risk`]),
//! located at `Section <its section>`. Nothing else is reported: every issue names a clause type
//! the document lacks or a section it has.

use crate::checklist::check_contract;
use crate::document::{Clause, RiskLevel};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// One issue of a contract
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    /// `issue-001`, `issue-002`, … in report order
    pub id: String,
    pub description: String,
    pub severity: RiskLevel,
    /// `document`, or `Section <section>` of the clause it is about
    pub location: String,
}

/// The issues of `clauses`: the missing required clause types (in checklist order), then the
/// clauses of high or critical risk (in document order)
#[must_use]
pub fn find_issues(clauses: &[Clause]) -> Vec<Issue> {
    let mut found: Vec<(String, RiskLevel, String)> = Vec::new();
    for item in check_contract(clauses) {
        if item.required && !item.found {
            found.push((
                format!("No {} clause found.", item.name),
                RiskLevel::Medium,
                String::from("document"),
            ));
        }
    }
    for c in clauses {
        let level = match c.risk_level {
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
            RiskLevel::Low | RiskLevel::Medium => continue,
        };
        found.push((
            format!("Clause {} uses {level}-risk wording.", c.section),
            c.risk_level,
            format!("Section {}", c.section),
        ));
    }
    found
        .into_iter()
        .enumerate()
        .map(|(i, (description, severity, location))| Issue {
            id: format!("issue-{:03}", i + 1),
            description,
            severity,
            location,
        })
        .collect()
}

//! `split_clauses` against fixed documents: the clause boundaries, sections and count are exact,
//! and `find_issues` reports only what the document holds.

use alice_legal_ai::{find_issues, split_clauses, Clause, ClauseType, LegalError};

fn sections(clauses: &[Clause]) -> Vec<(&str, &str)> {
    clauses
        .iter()
        .map(|c| (c.section.as_str(), c.text.as_str()))
        .collect()
}

const NUMBERED: &str = "\
1. Definitions. \"Agreement\" means this document.
2. Confidentiality. Each party keeps the other's information confidential.
2.1 The obligation survives
termination of this Agreement.
3. Governing law. This Agreement is governed by the laws of Japan.
";

#[test]
fn numbered_headings_split_into_their_clauses() {
    let clauses = split_clauses(NUMBERED).expect("clauses");
    assert_eq!(
        sections(&clauses),
        vec![
            ("1", "Definitions. \"Agreement\" means this document."),
            (
                "2",
                "Confidentiality. Each party keeps the other's information confidential."
            ),
            (
                "2.1",
                "The obligation survives termination of this Agreement."
            ),
            (
                "3",
                "Governing law. This Agreement is governed by the laws of Japan."
            ),
        ]
    );
    let ids: Vec<&str> = clauses.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        ["clause-001", "clause-002", "clause-003", "clause-004"]
    );
    assert_eq!(clauses[1].clause_type, ClauseType::Confidentiality);
    assert_eq!(clauses[3].clause_type, ClauseType::Governing);
}

#[test]
fn section_article_and_paragraph_sign_headings_are_headings() {
    let doc = "Section 4.2 The Supplier shall indemnify the Customer.\r\n\
               Article 5 Either party may terminate on material breach.\r\n\
               \u{a7} 6 The Supplier warrants the goods.\r\n";
    let clauses = split_clauses(doc).expect("clauses");
    assert_eq!(
        sections(&clauses),
        vec![
            ("4.2", "The Supplier shall indemnify the Customer."),
            ("5", "Either party may terminate on material breach."),
            ("6", "The Supplier warrants the goods."),
        ]
    );
    let types: Vec<ClauseType> = clauses.iter().map(|c| c.clause_type).collect();
    assert_eq!(
        types,
        [
            ClauseType::Indemnification,
            ClauseType::Termination,
            ClauseType::Warranty
        ]
    );
}

#[test]
fn without_headings_each_paragraph_is_a_clause() {
    let doc = "The parties agree as follows.\n\n\n  Payment is due within\n30 days.  \n\nThis Agreement is governed by the laws of Japan.";
    let clauses = split_clauses(doc).expect("clauses");
    assert_eq!(
        sections(&clauses),
        vec![
            ("\u{b6}1", "The parties agree as follows."),
            ("\u{b6}2", "Payment is due within 30 days."),
            (
                "\u{b6}3",
                "This Agreement is governed by the laws of Japan."
            ),
        ]
    );
}

#[test]
fn a_number_without_a_dot_does_not_start_a_clause() {
    let doc = "1. Term. The term begins in\n2026 and runs for 3 years.\n";
    let clauses = split_clauses(doc).expect("clauses");
    assert_eq!(
        sections(&clauses),
        vec![("1", "Term. The term begins in 2026 and runs for 3 years.")]
    );
}

#[test]
fn a_heading_without_text_is_not_a_clause() {
    let clauses =
        split_clauses("1.\n2. Termination. Either party may terminate.\n").expect("clauses");
    assert_eq!(
        sections(&clauses),
        vec![("2", "Termination. Either party may terminate.")]
    );
}

#[test]
fn an_empty_document_is_an_error() {
    assert_eq!(split_clauses(""), Err(LegalError::EmptyDocument));
    assert_eq!(split_clauses(" \n\t\r\n "), Err(LegalError::EmptyDocument));
}

#[test]
fn a_document_of_headings_only_has_no_clauses() {
    assert_eq!(
        split_clauses("1.\n2.\nSection 3\n"),
        Err(LegalError::NoClauses)
    );
}

#[test]
fn issues_name_only_what_the_document_holds() {
    // no indemnification clause in this document
    let clauses = split_clauses(NUMBERED).expect("clauses");
    let issues = find_issues(&clauses);
    let present: Vec<&str> = clauses.iter().map(|c| c.section.as_str()).collect();
    for issue in &issues {
        match issue.location.strip_prefix("Section ") {
            // an issue about a clause points at a clause the document has
            Some(section) => assert!(present.contains(&section), "{issue:?}"),
            // an issue about the whole document is a required clause type it lacks
            None => {
                assert_eq!(issue.location, "document", "{issue:?}");
                let missing = issue
                    .description
                    .strip_prefix("No ")
                    .and_then(|d| d.strip_suffix(" clause found."));
                let name =
                    missing.unwrap_or_else(|| panic!("not a missing-clause issue: {issue:?}"));
                assert!(
                    alice_legal_ai::check_contract(&clauses)
                        .iter()
                        .any(|i| i.name == name && !i.found),
                    "{issue:?}"
                );
            }
        }
    }
    // the indemnification issue is the missing clause, never a claim about an indemnification clause
    assert!(issues
        .iter()
        .any(|i| i.description == "No Indemnification clause found."));
    assert!(!issues.iter().any(|i| i
        .description
        .to_lowercase()
        .contains("indemnification clause detected")));
}

#[test]
fn a_risky_clause_is_reported_at_its_section() {
    let clauses = split_clauses("7. Liability. The Supplier accepts unlimited liability.\n")
        .expect("clauses");
    let issues = find_issues(&clauses);
    assert!(
        issues.iter().any(|i| i.location == "Section 7"),
        "{issues:?}"
    );
}

#[test]
fn issue_ids_are_numbered_in_order() {
    let clauses = split_clauses(NUMBERED).expect("clauses");
    let issues = find_issues(&clauses);
    assert!(!issues.is_empty());
    for (n, issue) in issues.iter().enumerate() {
        assert_eq!(issue.id, format!("issue-{:03}", n + 1));
    }
}

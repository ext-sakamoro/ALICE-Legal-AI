//! splitter: a document into its clauses.
//!
//! A clause starts at a heading line and runs to the next one. A heading is a line that starts
//! (after leading whitespace) with
//!
//! - a number with a dot: `1.`, `4.2`, `4.2.1.` (a single number needs the dot, so a line
//!   starting with a year such as `2026 and …` is not a heading);
//! - `Section`, `Article` or `Clause` (any case) and a number: `Section 4.2`;
//! - `§` and a number: `§ 6`.
//!
//! The number is the clause's section; the rest of the heading line and the lines up to the next
//! heading are its text, with every run of whitespace (line breaks included) as one space. A
//! heading with no text is not a clause. Text before the first heading is a clause of section
//! `preamble`. A document with no heading at all is split at blank lines, each paragraph a clause
//! of section `¶1`, `¶2`, …
//!
//! Each clause is classified ([`classify_clause`]) and its risk assessed ([`assess_risk`]); the ids
//! are `clause-001`, `clause-002`, … in document order.

use crate::classifier::{assess_risk, classify_clause};
use crate::document::Clause;
use crate::errors::LegalError;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// The clauses of `document`
///
/// # Errors
///
/// [`LegalError::EmptyDocument`] for a document of whitespace only, [`LegalError::NoClauses`]
/// for one whose headings carry no text.
pub fn split_clauses(document: &str) -> Result<Vec<Clause>, LegalError> {
    if document.trim().is_empty() {
        return Err(LegalError::EmptyDocument);
    }
    let lines: Vec<&str> = document.lines().collect();
    let parts = if lines.iter().any(|l| heading(l).is_some()) {
        by_heading(&lines)
    } else {
        by_paragraph(&lines)
    };
    let clauses: Vec<Clause> = parts
        .into_iter()
        .filter(|(_, text)| !text.is_empty())
        .enumerate()
        .map(|(i, (section, text))| Clause {
            id: format!("clause-{:03}", i + 1),
            section,
            clause_type: classify_clause(&text),
            risk_level: assess_risk(&text),
            text,
        })
        .collect();
    if clauses.is_empty() {
        return Err(LegalError::NoClauses);
    }
    Ok(clauses)
}

/// Every run of whitespace as one space, trimmed
fn collapse(parts: &[&str]) -> String {
    let mut out = String::new();
    for word in parts.iter().flat_map(|p| p.split_whitespace()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

fn by_heading(lines: &[&str]) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut section = "preamble".to_string();
    let mut body: Vec<&str> = Vec::new();
    for line in lines {
        if let Some((label, rest)) = heading(line) {
            parts.push((core::mem::replace(&mut section, label), collapse(&body)));
            body = Vec::from([rest]);
        } else {
            body.push(line);
        }
    }
    parts.push((section, collapse(&body)));
    parts
}

fn by_paragraph(lines: &[&str]) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut body: Vec<&str> = Vec::new();
    for line in lines.iter().chain(core::iter::once(&"")) {
        if line.trim().is_empty() {
            if !body.is_empty() {
                parts.push((format!("\u{b6}{}", parts.len() + 1), collapse(&body)));
                body.clear();
            }
        } else {
            body.push(line);
        }
    }
    parts
}

/// The section label and the rest of `line` when it is a heading
fn heading(line: &str) -> Option<(String, &str)> {
    let t = line.trim_start();
    for keyword in ["section", "article", "clause"] {
        if let Some(head) = t.get(..keyword.len()) {
            if head.eq_ignore_ascii_case(keyword) {
                let rest = &t[keyword.len()..];
                if rest.starts_with(char::is_whitespace) {
                    return number(rest.trim_start(), false);
                }
            }
        }
    }
    if let Some(rest) = t.strip_prefix('\u{a7}') {
        return number(rest.trim_start(), false);
    }
    number(t, true)
}

/// `digits(.digits)*`, an optional `.`, then whitespace or the end: (the number, the rest after
/// the whitespace); with `bare`, a single number needs the dot
fn number(s: &str, bare: bool) -> Option<(String, &str)> {
    let bytes = s.as_bytes();
    let mut end = 0;
    let mut components = 0;
    loop {
        let start = end;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == start {
            return None;
        }
        components += 1;
        if end + 1 < bytes.len() && bytes[end] == b'.' && bytes[end + 1].is_ascii_digit() {
            end += 1;
        } else {
            break;
        }
    }
    let label = &s[..end];
    let mut rest = &s[end..];
    let dotted = rest.starts_with('.');
    if dotted {
        rest = &rest[1..];
    }
    if bare && components == 1 && !dotted {
        return None;
    }
    if !(rest.is_empty() || rest.starts_with(char::is_whitespace)) {
        return None;
    }
    Some((label.to_string(), rest.trim()))
}

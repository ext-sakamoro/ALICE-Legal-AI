//! errors.

// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegalError {
    ParseError,
    ClassificationFailed,
    /// The document is empty or whitespace only
    EmptyDocument,
    /// The document's headings carry no text: there is no clause
    NoClauses,
}

impl core::fmt::Display for LegalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ParseError => write!(f, "parse error"),
            Self::ClassificationFailed => write!(f, "classification failed"),
            Self::EmptyDocument => write!(f, "the document is empty"),
            Self::NoClauses => write!(f, "the document has no clause"),
        }
    }
}

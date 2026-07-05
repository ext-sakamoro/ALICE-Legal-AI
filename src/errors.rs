//! errors.

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

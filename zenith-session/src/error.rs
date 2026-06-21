//! [`SessionError`]: the single error type for the zenith-session crate.

use std::fmt;

/// An error produced by zenith-session operations.
///
/// Mirrors the hand-rolled style of `zenith-tx`'s `TxError` — no third-party
/// error libraries.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionError {
    pub message: String,
}

impl SessionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "zenith-session error: {}", self.message)
    }
}

impl std::error::Error for SessionError {}

impl From<std::io::Error> for SessionError {
    fn from(e: std::io::Error) -> Self {
        Self::new(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_message() {
        let e = SessionError::new("something went wrong");
        assert_eq!(e.to_string(), "zenith-session error: something went wrong");
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let e = SessionError::from(io_err);
        assert!(e.message.contains("file missing"));
    }
}

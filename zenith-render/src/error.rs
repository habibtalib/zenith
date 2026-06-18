//! Error type for the render crate.

/// An error produced by the rasterization or encoding pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderError {
    /// Human-readable description of what went wrong.
    pub message: String,
}

impl RenderError {
    /// Construct a `RenderError` with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RenderError {}

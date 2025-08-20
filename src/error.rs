use thiserror::Error;
use std::io;

/// Custom error types for the Tiger Cache library
#[derive(Error, Debug)]
pub enum TigerCacheError {
    /// Error during serialization or deserialization
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Error during JSON processing
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// I/O error during file operations
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// Document not found in the index
    #[error("Document with ID {0} not found")]
    DocumentNotFound(String),

    /// Invalid document format
    #[error("Invalid document format: {0}")]
    InvalidDocument(String),

    /// Invalid search query
    #[error("Invalid search query: {0}")]
    InvalidQuery(String),
}

/// Result type alias for Tiger Cache operations
pub type Result<T> = std::result::Result<T, TigerCacheError>;


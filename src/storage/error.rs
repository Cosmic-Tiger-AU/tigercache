use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Storage-specific error types
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Storage path not found: {0}")]
    StoragePathNotFound(PathBuf),

    #[error("Unsupported storage type: {0}")]
    UnsupportedStorageType(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Page error: {0}")]
    PageError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Storage already exists at path: {0}")]
    StorageAlreadyExists(PathBuf),

    #[error("Storage is corrupted: {0}")]
    StorageCorrupted(String),

    #[error("Storage version mismatch: expected {expected}, found {found}")]
    StorageVersionMismatch { expected: String, found: String },

    #[error("Storage is locked by another process")]
    StorageLocked,

    #[error("Storage operation timeout")]
    StorageTimeout,

    #[error("Storage operation canceled")]
    StorageCanceled,

    #[error("Storage operation not supported: {0}")]
    StorageOperationNotSupported(String),

    #[error("Storage error: {0}")]
    Other(String),
}

// Implement conversions from backend-specific errors
#[cfg(feature = "sled-storage")]
impl From<sled::Error> for StorageError {
    fn from(err: sled::Error) -> Self {
        match err {
            sled::Error::Io(io_err) => StorageError::IoError(io_err),
            sled::Error::Corruption { .. } => StorageError::StorageCorrupted(err.to_string()),
            sled::Error::CollectionNotFound(_) => StorageError::KeyNotFound(err.to_string()),
            sled::Error::Unsupported(_) => StorageError::StorageOperationNotSupported(err.to_string()),
            sled::Error::ReportableBug(_) => StorageError::DatabaseError(err.to_string()),
            _ => StorageError::Other(err.to_string()),
        }
    }
}

#[cfg(feature = "redb-storage")]
impl From<redb::Error> for StorageError {
    fn from(err: redb::Error) -> Self {
        match err {
            redb::Error::Io(io_err) => StorageError::IoError(io_err),
            redb::Error::Corrupted(_) => StorageError::StorageCorrupted(err.to_string()),
            redb::Error::TableNotFound(_) => StorageError::KeyNotFound(err.to_string()),
            redb::Error::InvalidKey => StorageError::KeyNotFound("Invalid key".to_string()),
            redb::Error::TransactionAborted => StorageError::TransactionError("Transaction aborted".to_string()),
            _ => StorageError::Other(err.to_string()),
        }
    }
}

#[cfg(feature = "rocksdb-storage")]
impl From<rocksdb::Error> for StorageError {
    fn from(err: rocksdb::Error) -> Self {
        StorageError::DatabaseError(err.to_string())
    }
}

impl From<bincode::error::EncodeError> for StorageError {
    fn from(err: bincode::error::EncodeError) -> Self {
        StorageError::SerializationError(err.to_string())
    }
}

impl From<bincode::error::DecodeError> for StorageError {
    fn from(err: bincode::error::DecodeError) -> Self {
        StorageError::DeserializationError(err.to_string())
    }
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;


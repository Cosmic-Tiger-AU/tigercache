// Storage module for TigerCache
//
// This module provides the storage backends for TigerCache, allowing it to
// operate in a hybrid memory/disk mode similar to SQLite.

mod error;
mod config;
mod page;
mod storage_engine;

// Storage backends
#[cfg(feature = "sled-storage")]
mod sled_engine;
#[cfg(feature = "redb-storage")]
mod redb_engine;
#[cfg(feature = "rocksdb-storage")]
mod rocksdb_engine;
mod sqlite_engine;

// Re-exports
pub use error::{StorageError, StorageResult};
pub use config::{StorageConfig, StorageType};
pub use page::{Page, PageId};
pub use storage_engine::{StorageEngine, StorageTransaction, StorageStats};

// Factory function to create a storage engine based on configuration
pub fn create_storage_engine(config: StorageConfig) -> StorageResult<Box<dyn StorageEngine>> {
    match config.storage_type {
        #[cfg(feature = "sled-storage")]
        StorageType::Sled => {
            let engine = sled_engine::SledStorageEngine::new(config)?;
            Ok(Box::new(engine))
        },
        #[cfg(feature = "redb-storage")]
        StorageType::Redb => {
            let engine = redb_engine::RedbStorageEngine::new(config)?;
            Ok(Box::new(engine))
        },
        #[cfg(feature = "rocksdb-storage")]
        StorageType::RocksDB => {
            let engine = rocksdb_engine::RocksDBStorageEngine::new(config)?;
            Ok(Box::new(engine))
        },
        StorageType::SQLite => {
            let engine = sqlite_engine::SqliteStorageEngine::new(config)?;
            Ok(Box::new(engine))
        },
        StorageType::Memory => {
            // For testing or small datasets, we can use an in-memory storage engine
            let engine = storage_engine::MemoryStorageEngine::new(config)?;
            Ok(Box::new(engine))
        },
        #[allow(unreachable_patterns)]
        _ => Err(StorageError::UnsupportedStorageType(format!("{:?}", config.storage_type))),
    }
}

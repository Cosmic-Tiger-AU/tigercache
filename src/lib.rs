//! # Tiger Cache
//!
//! Tiger Cache is an embedded fuzzy search library inspired by Typesense.
//! It provides fast, typo-tolerant search over a local cache of documents.
//!
//! ## Features
//!
//! - Embedded search engine with no external dependencies
//! - Fuzzy search with trigram indexing and Levenshtein distance
//! - Persistence to a single file
//! - Simple, intuitive API
//!
//! ## Example
//!
//! ```rust
//! use tiger_cache::{Document, TigerCache};
//!
//! // Create a new search engine
//! let mut tiger_cache = TigerCache::new();
//!
//! // Add documents
//! let mut doc = Document::new("doc1");
//! doc.add_field("title", "Apple iPhone")
//!    .add_field("description", "The latest smartphone from Apple");
//! tiger_cache.add_document(doc).unwrap();
//!
//! // Search
//! let results = tiger_cache.search("iphone", None).unwrap();
//! for result in results {
//!     println!("Found: {} (score: {})", result.document.id, result.score);
//! }
//!
//! // Save to file
//! tiger_cache.save_to_file("search_index.bin").unwrap();
//! ```

mod document;
mod error;
mod index;
mod intern;
mod tiger_cache;
mod persistence;
mod search;
mod trigram;
mod storage;
mod config;
mod cache;

// Re-export public API
pub use document::Document;
pub use error::{TigerCacheError, Result};
pub use tiger_cache::TigerCache;
pub use search::{SearchOptions, SearchResult};
pub use config::TigerCacheConfig;

// Re-export storage API
pub use storage::{
    StorageConfig,
    StorageType,
    StorageEngine,
    StorageTransaction,
    create_storage_engine,
    StorageError,
    StorageResult,
};

// Re-export cache API
pub use cache::{
    MemoryManager,
    MemoryStats,
};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
mod tiger_cache;
mod persistence;
mod search;
mod trigram;

// Re-export public API
pub use document::Document;
pub use error::{TigerCacheError, Result};
pub use tiger_cache::TigerCache;
pub use search::{SearchOptions, SearchResult};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

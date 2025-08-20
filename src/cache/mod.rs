// Cache module for TigerCache
//
// This module provides memory management and caching for TigerCache,
// allowing it to efficiently manage memory usage while maintaining
// performance.

mod memory_manager;
mod lru_cache;
mod document_cache;
mod index_cache;
mod query_cache;

// Re-exports
pub use memory_manager::{MemoryManager, MemoryStats};
pub use lru_cache::LruCache;
pub use document_cache::DocumentCache;
pub use index_cache::IndexCache;
pub use query_cache::QueryCache;


use std::sync::Arc;
use bytesize::ByteSize;
use smallvec::SmallVec;

use crate::cache::lru_cache::LruCache;
use crate::intern::StringId;

/// Index cache for TigerCache
///
/// Caches index data to avoid repeated lookups.
pub struct IndexCache {
    /// LRU cache for token to document ID mappings
    token_to_docs: LruCache<StringId, Vec<StringId>>,
    
    /// LRU cache for trigram to token ID mappings
    trigram_to_tokens: LruCache<StringId, Vec<StringId>>,
}

impl IndexCache {
    /// Create a new index cache with the specified maximum size
    pub fn new(max_size: ByteSize) -> Self {
        // Split the cache size between token and trigram caches
        let token_cache_size = max_size / 2;
        let trigram_cache_size = max_size - token_cache_size;
        
        Self {
            token_to_docs: LruCache::new(token_cache_size),
            trigram_to_tokens: LruCache::new(trigram_cache_size),
        }
    }
    
    /// Get document IDs for a token
    pub fn get_docs_for_token(&self, token_id: StringId) -> Option<Vec<StringId>> {
        self.token_to_docs.get(&token_id)
    }
    
    /// Put document IDs for a token in the cache
    pub fn put_docs_for_token(&self, token_id: StringId, doc_ids: Vec<StringId>) -> Option<Vec<StringId>> {
        let size = estimate_vec_size(&doc_ids);
        self.token_to_docs.put(token_id, doc_ids, size)
    }
    
    /// Get token IDs for a trigram
    pub fn get_tokens_for_trigram(&self, trigram_id: StringId) -> Option<Vec<StringId>> {
        self.trigram_to_tokens.get(&trigram_id)
    }
    
    /// Put token IDs for a trigram in the cache
    pub fn put_tokens_for_trigram(&self, trigram_id: StringId, token_ids: Vec<StringId>) -> Option<Vec<StringId>> {
        let size = estimate_vec_size(&token_ids);
        self.trigram_to_tokens.put(trigram_id, token_ids, size)
    }
    
    /// Clear the cache
    pub fn clear(&self) {
        self.token_to_docs.clear();
        self.trigram_to_tokens.clear();
    }
    
    /// Get the current size of the cache in bytes
    pub fn size(&self) -> ByteSize {
        self.token_to_docs.size() + self.trigram_to_tokens.size()
    }
    
    /// Get the maximum size of the cache in bytes
    pub fn max_size(&self) -> ByteSize {
        self.token_to_docs.max_size() + self.trigram_to_tokens.max_size()
    }
    
    /// Get the number of entries in the cache
    pub fn len(&self) -> usize {
        self.token_to_docs.len() + self.trigram_to_tokens.len()
    }
    
    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.token_to_docs.is_empty() && self.trigram_to_tokens.is_empty()
    }
}

/// Estimate the size of a vector in bytes
fn estimate_vec_size<T>(vec: &[T]) -> usize {
    let mut size = std::mem::size_of::<Vec<T>>();
    
    // Add the size of the elements
    size += vec.len() * std::mem::size_of::<T>();
    
    size
}


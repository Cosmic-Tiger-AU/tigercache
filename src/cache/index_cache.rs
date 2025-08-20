use std::sync::Arc;
use bytesize::ByteSize;
use smallvec::SmallVec;

use crate::cache::lru_cache::LruCache;
use crate::intern::StringId;

/// Index cache for TigerCache
///
/// Caches index segments to avoid repeated disk access.
pub struct IndexCache {
    /// LRU cache for trigram index entries
    trigram_cache: LruCache<StringId, SmallVec<[StringId; 4]>>,
    
    /// LRU cache for inverted index entries
    inverted_cache: LruCache<StringId, SmallVec<[StringId; 8]>>,
}

impl IndexCache {
    /// Create a new index cache with the specified maximum sizes
    pub fn new(trigram_cache_size: ByteSize, inverted_cache_size: ByteSize) -> Self {
        Self {
            trigram_cache: LruCache::new(trigram_cache_size),
            inverted_cache: LruCache::new(inverted_cache_size),
        }
    }
    
    /// Get a trigram index entry from the cache
    pub fn get_trigram(&self, trigram_id: StringId) -> Option<SmallVec<[StringId; 4]>> {
        self.trigram_cache.get(&trigram_id)
    }
    
    /// Put a trigram index entry in the cache
    pub fn put_trigram(&self, trigram_id: StringId, tokens: SmallVec<[StringId; 4]>) -> Option<SmallVec<[StringId; 4]>> {
        let size = estimate_smallvec_size(&tokens);
        self.trigram_cache.put(trigram_id, tokens, size)
    }
    
    /// Get an inverted index entry from the cache
    pub fn get_inverted(&self, token_id: StringId) -> Option<SmallVec<[StringId; 8]>> {
        self.inverted_cache.get(&token_id)
    }
    
    /// Put an inverted index entry in the cache
    pub fn put_inverted(&self, token_id: StringId, doc_ids: SmallVec<[StringId; 8]>) -> Option<SmallVec<[StringId; 8]>> {
        let size = estimate_smallvec_size(&doc_ids);
        self.inverted_cache.put(token_id, doc_ids, size)
    }
    
    /// Clear the cache
    pub fn clear(&self) {
        self.trigram_cache.clear();
        self.inverted_cache.clear();
    }
    
    /// Get the current size of the trigram cache in bytes
    pub fn trigram_size(&self) -> ByteSize {
        self.trigram_cache.size()
    }
    
    /// Get the current size of the inverted cache in bytes
    pub fn inverted_size(&self) -> ByteSize {
        self.inverted_cache.size()
    }
    
    /// Get the total size of the cache in bytes
    pub fn total_size(&self) -> ByteSize {
        ByteSize::b(self.trigram_cache.size().as_u64() + self.inverted_cache.size().as_u64())
    }
    
    /// Get the trigram cache hit rate (0.0 - 1.0)
    pub fn trigram_hit_rate(&self) -> f64 {
        self.trigram_cache.hit_rate()
    }
    
    /// Get the inverted cache hit rate (0.0 - 1.0)
    pub fn inverted_hit_rate(&self) -> f64 {
        self.inverted_cache.hit_rate()
    }
    
    /// Get the average cache hit rate (0.0 - 1.0)
    pub fn average_hit_rate(&self) -> f64 {
        (self.trigram_cache.hit_rate() + self.inverted_cache.hit_rate()) / 2.0
    }
}

/// Estimate the size of a SmallVec in bytes
fn estimate_smallvec_size<T, const N: usize>(vec: &SmallVec<[T; N]>) -> usize {
    // Base size for the SmallVec struct
    let mut size = std::mem::size_of::<SmallVec<[T; N]>>();
    
    // Add the size of the elements
    size += vec.len() * std::mem::size_of::<T>();
    
    size
}


use std::sync::Arc;
use bytesize::ByteSize;

use crate::cache::lru_cache::LruCache;
use crate::search::SearchResult;

/// Query cache for TigerCache
///
/// Caches search results to avoid repeated searches.
pub struct QueryCache {
    /// LRU cache for search results
    cache: LruCache<String, Vec<SearchResult>>,
}

impl QueryCache {
    /// Create a new query cache with the specified maximum size
    pub fn new(max_size: ByteSize) -> Self {
        Self {
            cache: LruCache::new(max_size),
        }
    }
    
    /// Get search results from the cache
    pub fn get(&self, query: &str) -> Option<Vec<SearchResult>> {
        self.cache.get(query)
    }
    
    /// Put search results in the cache
    pub fn put(&self, query: String, results: Vec<SearchResult>) -> Option<Vec<SearchResult>> {
        // Estimate the size of the results
        let size = estimate_results_size(&results);
        self.cache.put(query, results, size)
    }
    
    /// Clear the cache
    pub fn clear(&self) {
        self.cache.clear();
    }
    
    /// Get the current size of the cache in bytes
    pub fn size(&self) -> ByteSize {
        self.cache.size()
    }
    
    /// Get the maximum size of the cache in bytes
    pub fn max_size(&self) -> ByteSize {
        self.cache.max_size()
    }
    
    /// Get the number of entries in the cache
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    
    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

/// Estimate the size of search results in bytes
fn estimate_results_size(results: &[SearchResult]) -> usize {
    let mut size = 0;
    
    // Base size of the vector
    size += std::mem::size_of::<Vec<SearchResult>>();
    
    // Size of each result
    for result in results {
        // Size of the document
        size += std::mem::size_of::<String>() + result.document.id.len();
        
        // Size of the fields
        for (key, value) in &result.document.fields {
            size += std::mem::size_of::<String>() + key.len();
            
            // Estimate the size of the JSON value
            match value {
                serde_json::Value::String(s) => {
                    size += std::mem::size_of::<String>() + s.len();
                },
                serde_json::Value::Number(_) => {
                    size += std::mem::size_of::<f64>();
                },
                serde_json::Value::Bool(_) => {
                    size += std::mem::size_of::<bool>();
                },
                serde_json::Value::Array(arr) => {
                    size += std::mem::size_of::<Vec<serde_json::Value>>();
                    size += arr.len() * std::mem::size_of::<serde_json::Value>();
                },
                serde_json::Value::Object(obj) => {
                    size += std::mem::size_of::<serde_json::Map<String, serde_json::Value>>();
                    size += obj.len() * (std::mem::size_of::<String>() + std::mem::size_of::<serde_json::Value>());
                },
                serde_json::Value::Null => {
                    // No additional size
                },
            }
        }
        
        // Size of the score
        size += std::mem::size_of::<f64>();
        
        // Size of the matched fields
        size += std::mem::size_of::<Vec<String>>();
        for field in &result.matched_fields {
            size += std::mem::size_of::<String>() + field.len();
        }
    }
    
    size
}


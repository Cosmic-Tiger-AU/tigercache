use std::sync::Arc;
use bytesize::ByteSize;

use crate::search::{SearchOptions, SearchResult};
use crate::cache::lru_cache::LruCache;

/// Query key for cache lookups
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct QueryKey {
    /// Query string
    query: String,
    
    /// Search options
    options: Option<SearchOptions>,
}

/// Query cache for TigerCache
///
/// Caches search results to avoid repeated computation.
pub struct QueryCache {
    /// LRU cache for query results
    cache: LruCache<QueryKey, Arc<Vec<SearchResult>>>,
}

impl QueryCache {
    /// Create a new query cache with the specified maximum size
    pub fn new(max_size: ByteSize) -> Self {
        Self {
            cache: LruCache::new(max_size),
        }
    }
    
    /// Get search results from the cache
    pub fn get(&self, query: &str, options: Option<&SearchOptions>) -> Option<Arc<Vec<SearchResult>>> {
        let key = QueryKey {
            query: query.to_string(),
            options: options.cloned(),
        };
        
        self.cache.get(&key)
    }
    
    /// Put search results in the cache
    pub fn put(&self, query: &str, options: Option<&SearchOptions>, results: Vec<SearchResult>) -> Option<Arc<Vec<SearchResult>>> {
        let key = QueryKey {
            query: query.to_string(),
            options: options.cloned(),
        };
        
        let size = estimate_results_size(&results);
        let results_arc = Arc::new(results);
        
        self.cache.put(key, results_arc.clone(), size)
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
    
    /// Get the number of queries in the cache
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    
    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
    
    /// Get the cache hit rate (0.0 - 1.0)
    pub fn hit_rate(&self) -> f64 {
        self.cache.hit_rate()
    }
}

/// Estimate the size of search results in bytes
fn estimate_results_size(results: &[SearchResult]) -> usize {
    // Base size for the Vec struct
    let mut size = std::mem::size_of::<Vec<SearchResult>>();
    
    // Add the size of each result
    for result in results {
        // Add the size of the SearchResult struct
        size += std::mem::size_of::<SearchResult>();
        
        // Add the size of the document
        size += estimate_document_size(&result.document);
    }
    
    size
}

/// Estimate the size of a document in bytes
fn estimate_document_size(document: &crate::document::Document) -> usize {
    // Base size for the document struct
    let mut size = std::mem::size_of::<crate::document::Document>();
    
    // Add the size of the document ID
    size += document.id.len();
    
    // Add the size of each field
    for (key, value) in &document.fields {
        // Add the size of the key
        size += key.len();
        
        // Add the size of the value
        match value {
            crate::document::FieldValue::Text(text) => {
                size += text.len();
            }
            crate::document::FieldValue::Number(_) => {
                size += std::mem::size_of::<f64>();
            }
            crate::document::FieldValue::Boolean(_) => {
                size += std::mem::size_of::<bool>();
            }
        }
    }
    
    size
}


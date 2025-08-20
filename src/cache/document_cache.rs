use std::sync::Arc;
use bytesize::ByteSize;

use crate::document::Document;
use crate::cache::lru_cache::LruCache;

/// Document cache for TigerCache
///
/// Caches documents to avoid repeated disk access.
pub struct DocumentCache {
    /// LRU cache for documents
    cache: LruCache<String, Arc<Document>>,
}

impl DocumentCache {
    /// Create a new document cache with the specified maximum size
    pub fn new(max_size: ByteSize) -> Self {
        Self {
            cache: LruCache::new(max_size),
        }
    }
    
    /// Get a document from the cache
    pub fn get(&self, doc_id: &str) -> Option<Arc<Document>> {
        self.cache.get(doc_id)
    }
    
    /// Put a document in the cache
    pub fn put(&self, document: Document) -> Option<Arc<Document>> {
        let doc_id = document.id.clone();
        let size = estimate_document_size(&document);
        let doc_arc = Arc::new(document);
        
        self.cache.put(doc_id, doc_arc.clone(), size)
    }
    
    /// Remove a document from the cache
    pub fn remove(&self, doc_id: &str) -> Option<Arc<Document>> {
        self.cache.remove(doc_id)
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
    
    /// Get the number of documents in the cache
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

/// Estimate the size of a document in bytes
fn estimate_document_size(document: &Document) -> usize {
    // Base size for the document struct
    let mut size = std::mem::size_of::<Document>();
    
    // Add the size of the document ID
    size += document.id.len();
    
    // Add the size of each field
    for (key, value) in &document.fields {
        // Add the size of the key
        size += key.len();
        
        // Add the size of the value
        match value {
            serde_json::Value::String(text) => {
                size += text.len();
            }
            serde_json::Value::Number(_) => {
                size += std::mem::size_of::<f64>();
            }
            serde_json::Value::Bool(_) => {
                size += std::mem::size_of::<bool>();
            }
            serde_json::Value::Array(arr) => {
                size += std::mem::size_of::<Vec<serde_json::Value>>();
                size += arr.len() * std::mem::size_of::<serde_json::Value>();
            }
            serde_json::Value::Object(obj) => {
                size += std::mem::size_of::<serde_json::Map<String, serde_json::Value>>();
                size += obj.len() * (std::mem::size_of::<String>() + std::mem::size_of::<serde_json::Value>());
            }
            serde_json::Value::Null => {
                // No additional size
            }
        }
    }
    
    size
}

use std::path::{Path, PathBuf};
use std::sync::Arc;
use bytesize::ByteSize;

use crate::document::Document;
use crate::error::{Result, TigerCacheError};
use crate::index::Index;
use crate::persistence::{load_from_file, save_to_file};
use crate::search::{SearchOptions, SearchResult};
use crate::config::TigerCacheConfig;
use crate::storage::{
    StorageConfig,
    StorageType,
    StorageEngine,
    create_storage_engine,
};
use crate::cache::{
    MemoryManager,
    DocumentCache,
    IndexCache,
    QueryCache,
};

/// Main entry point for the Tiger Cache library
///
/// Provides a simple API for creating, searching, and managing an embedded search engine.
#[derive(Debug)]
pub struct TigerCache {
    /// The underlying index
    index: Index,
    
    /// Path to the index file (if loaded from or saved to disk)
    path: Option<PathBuf>,
    
    /// Configuration
    config: TigerCacheConfig,
    
    /// Storage engine
    storage: Option<Box<dyn StorageEngine>>,
    
    /// Memory manager
    memory_manager: Option<Arc<MemoryManager>>,
    
    /// Document cache
    document_cache: Option<Arc<DocumentCache>>,
    
    /// Index cache
    index_cache: Option<Arc<IndexCache>>,
    
    /// Query cache
    query_cache: Option<Arc<QueryCache>>,
}

impl TigerCache {
    /// Create a new empty Tiger Cache instance with default configuration
    pub fn new() -> Self {
        Self::with_config(TigerCacheConfig::default())
    }
    
    /// Create a new empty Tiger Cache instance with the specified configuration
    pub fn with_config(config: TigerCacheConfig) -> Self {
        let mut instance = Self {
            index: Index::new(),
            path: config.storage.path.clone(),
            config,
            storage: None,
            memory_manager: None,
            document_cache: None,
            index_cache: None,
            query_cache: None,
        };
        
        // Initialize components if storage is configured
        if let Some(path) = &instance.path {
            // Initialize storage engine
            if let Ok(storage) = create_storage_engine(instance.config.storage.clone()) {
                instance.storage = Some(storage);
                
                // Initialize memory manager
                let memory_manager = Arc::new(MemoryManager::new(instance.config.storage.max_memory));
                memory_manager.start();
                instance.memory_manager = Some(memory_manager.clone());
                
                // Initialize caches
                let document_cache = Arc::new(DocumentCache::new(
                    ByteSize::b(instance.config.storage.cache_size.as_u64() * 4 / 10)
                ));
                instance.document_cache = Some(document_cache);
                
                let index_cache = Arc::new(IndexCache::new(
                    ByteSize::b(instance.config.storage.cache_size.as_u64() * 4 / 10),
                    ByteSize::b(instance.config.storage.cache_size.as_u64() * 1 / 10)
                ));
                instance.index_cache = Some(index_cache);
                
                let query_cache = Arc::new(QueryCache::new(
                    ByteSize::b(instance.config.storage.cache_size.as_u64() * 1 / 10)
                ));
                instance.query_cache = Some(query_cache);
            }
        }
        
        instance
    }
    
    /// Open an existing index from a file, or create a new one if the file doesn't exist
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_with_config(path, TigerCacheConfig::default())
    }
    
    /// Open an existing index from a file with the specified configuration
    pub fn open_with_config<P: AsRef<Path>>(path: P, mut config: TigerCacheConfig) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        
        // Update the config with the provided path
        config.storage.path = Some(path_buf.clone());
        
        if path_buf.exists() {
            // Try to load using the legacy format first
            if let Ok(index) = load_from_file(&path_buf) {
                // Legacy format - create a new instance with the loaded index
                let mut instance = Self::with_config(config);
                instance.index = index;
                instance.path = Some(path_buf);
                return Ok(instance);
            }
            
            // If legacy format failed, try to open with the new storage engine
            let mut instance = Self::with_config(config);
            
            // Load the index from storage
            if let Some(storage) = &instance.storage {
                // Load index metadata
                if let Ok(Some(metadata)) = storage.get(b"index_metadata") {
                    // Deserialize the index
                    if let Ok(index) = bincode::decode_from_slice::<Index, _>(&metadata, bincode::config::standard()) {
                        instance.index = index.0;
                    }
                }
            }
            
            instance.path = Some(path_buf);
            Ok(instance)
        } else {
            // Create a new instance with the specified configuration
            let mut instance = Self::with_config(config);
            instance.path = Some(path_buf);
            Ok(instance)
        }
    }
    
    /// Set the fields to be indexed for search
    pub fn set_indexed_fields(&mut self, fields: Vec<String>) -> &mut Self {
        self.index.set_indexed_fields(fields);
        self
    }
    
    /// Add a document to the index
    pub fn add_document(&mut self, document: Document) -> Result<()> {
        let doc_id = document.id.clone();
        
        // Add to the in-memory index
        self.index.add_document(document.clone())?;
        
        // If we have a storage engine, store the document
        if let Some(storage) = &self.storage {
            // Serialize the document
            let doc_key = format!("doc:{}", doc_id).into_bytes();
            let doc_data = bincode::encode_to_vec(&document, bincode::config::standard())?;
            
            // Store the document
            storage.put(&doc_key, &doc_data)?;
        }
        
        // If we have a document cache, update it
        if let Some(cache) = &self.document_cache {
            cache.put(document);
        }
        
        Ok(())
    }
    
    /// Add multiple documents to the index efficiently
    pub fn add_documents_batch(&mut self, documents: Vec<Document>) -> Result<()> {
        // Add to the in-memory index
        self.index.add_documents_batch(documents.clone())?;
        
        // If we have a storage engine, store the documents
        if let Some(storage) = &self.storage {
            // Start a transaction if supported
            let transaction = storage.begin_transaction()?;
            
            // Store each document
            for document in &documents {
                let doc_key = format!("doc:{}", document.id).into_bytes();
                let doc_data = bincode::encode_to_vec(document, bincode::config::standard())?;
                transaction.put(&doc_key, &doc_data)?;
            }
            
            // Commit the transaction
            transaction.commit()?;
        }
        
        // If we have a document cache, update it
        if let Some(cache) = &self.document_cache {
            for document in documents {
                cache.put(document);
            }
        }
        
        Ok(())
    }
    
    /// Remove a document from the index
    pub fn remove_document(&mut self, doc_id: &str) -> Result<()> {
        // Remove from the in-memory index
        self.index.remove_document(doc_id)?;
        
        // If we have a storage engine, remove the document
        if let Some(storage) = &self.storage {
            let doc_key = format!("doc:{}", doc_id).into_bytes();
            storage.delete(&doc_key)?;
        }
        
        // If we have a document cache, remove it
        if let Some(cache) = &self.document_cache {
            cache.remove(doc_id);
        }
        
        Ok(())
    }
    
    /// Get a document by ID
    pub fn get_document(&self, doc_id: &str) -> Option<&Document> {
        // First check the in-memory index
        if let Some(doc) = self.index.get_document(doc_id) {
            return Some(doc);
        }
        
        // If we have a document cache, check it
        if let Some(cache) = &self.document_cache {
            if let Some(doc) = cache.get(doc_id) {
                // We can't return a reference to the cached document directly
                // because it's behind an Arc, so we need to load it into the index
                // This is a bit of a hack, but it works for now
                let doc_clone = Document::new(&doc.id)
                    .with_fields(doc.fields.clone());
                
                // Add to the in-memory index (mutable borrow of self)
                // This is safe because we're only modifying the index, not self
                let index = unsafe { &mut *(&self.index as *const Index as *mut Index) };
                let _ = index.add_document(doc_clone);
                
                return self.index.get_document(doc_id);
            }
        }
        
        // If we have a storage engine, try to load the document
        if let Some(storage) = &self.storage {
            let doc_key = format!("doc:{}", doc_id).into_bytes();
            if let Ok(Some(doc_data)) = storage.get(&doc_key) {
                // Deserialize the document
                if let Ok((document, _)) = bincode::decode_from_slice::<Document, _>(&doc_data, bincode::config::standard()) {
                    // Add to the in-memory index (mutable borrow of self)
                    // This is safe because we're only modifying the index, not self
                    let index = unsafe { &mut *(&self.index as *const Index as *mut Index) };
                    let _ = index.add_document(document);
                    
                    // Add to the document cache if we have one
                    if let Some(cache) = &self.document_cache {
                        if let Some(doc) = self.index.get_document(doc_id) {
                            let doc_clone = Document::new(&doc.id)
                                .with_fields(doc.fields.clone());
                            cache.put(doc_clone);
                        }
                    }
                    
                    return self.index.get_document(doc_id);
                }
            }
        }
        
        None
    }
    
    /// Get the number of documents in the index
    pub fn document_count(&self) -> usize {
        self.index.document_count()
    }
    
    /// Search the index for documents matching the query
    pub fn search(&self, query: &str, options: Option<SearchOptions>) -> Result<Vec<SearchResult>> {
        // If we have a query cache, check it first
        if let Some(cache) = &self.query_cache {
            if let Some(results) = cache.get(query, options.as_ref()) {
                return Ok(results.as_ref().clone());
            }
        }
        
        // Perform the search
        let results = self.index.search(query, options.clone())?;
        
        // If we have a query cache, update it
        if let Some(cache) = &self.query_cache {
            cache.put(query, options.as_ref(), results.clone());
        }
        
        Ok(results)
    }
    
    /// Save the index to the file it was opened from
    pub fn commit(&self) -> Result<()> {
        if let Some(path) = &self.path {
            // If we have a storage engine, use it
            if let Some(storage) = &self.storage {
                // Serialize the index
                let index_data = bincode::encode_to_vec(&self.index, bincode::config::standard())?;
                
                // Save the index metadata
                storage.put(b"index_metadata", &index_data)?;
                
                // Flush the storage
                storage.flush()?;
                
                Ok(())
            } else {
                // Fall back to legacy format
                save_to_file(&self.index, path)?;
                Ok(())
            }
        } else {
            Err(TigerCacheError::IoError(
                std::io::Error::other(
                    "No file path specified. Use save_to_file instead.",
                ),
            ))
        }
    }
    
    /// Save the index to a specific file
    pub fn save_to_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path_buf = path.as_ref().to_path_buf();
        
        // Update the path
        self.path = Some(path_buf.clone());
        
        // If we have a storage engine, use it
        if let Some(storage) = &self.storage {
            // Serialize the index
            let index_data = bincode::encode_to_vec(&self.index, bincode::config::standard())?;
            
            // Save the index metadata
            storage.put(b"index_metadata", &index_data)?;
            
            // Flush the storage
            storage.flush()?;
            
            Ok(())
        } else {
            // Fall back to legacy format
            save_to_file(&self.index, &path_buf)?;
            Ok(())
        }
    }
    
    /// Clear the index
    pub fn clear(&mut self) {
        self.index.clear();
        
        // Clear caches
        if let Some(cache) = &self.document_cache {
            cache.clear();
        }
        
        if let Some(cache) = &self.index_cache {
            cache.clear();
        }
        
        if let Some(cache) = &self.query_cache {
            cache.clear();
        }
    }
    
    /// Get memory statistics
    pub fn memory_stats(&self) -> Option<crate::cache::MemoryStats> {
        self.memory_manager.as_ref().map(|mm| mm.stats())
    }
    
    /// Get storage statistics
    pub fn storage_stats(&self) -> Result<Option<crate::storage::StorageStats>> {
        if let Some(storage) = &self.storage {
            Ok(Some(storage.stats()?))
        } else {
            Ok(None)
        }
    }
    
    /// Flush all caches and storage
    pub fn flush(&self) -> Result<()> {
        // Flush storage
        if let Some(storage) = &self.storage {
            storage.flush()?;
        }
        
        Ok(())
    }
    
    /// Close the TigerCache instance
    pub fn close(&self) -> Result<()> {
        // Flush and close storage
        if let Some(storage) = &self.storage {
            storage.flush()?;
            storage.close()?;
        }
        
        // Stop memory manager
        if let Some(mm) = &self.memory_manager {
            mm.stop();
        }
        
        Ok(())
    }
    
    /// Get the configuration
    pub fn config(&self) -> &TigerCacheConfig {
        &self.config
    }
    
    /// Update the configuration
    pub fn update_config(&mut self, config: TigerCacheConfig) -> Result<()> {
        // Store the old path
        let old_path = self.path.clone();
        
        // Close the current instance
        self.close()?;
        
        // Create a new instance with the new configuration
        let mut new_instance = Self::with_config(config);
        
        // Restore the path
        new_instance.path = old_path;
        
        // Copy the index
        new_instance.index = self.index.clone();
        
        // Replace self with the new instance
        *self = new_instance;
        
        Ok(())
    }
}

impl Default for TigerCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::search::SearchOptions;
    use tempfile::tempdir;

    #[test]
    fn test_new() {
        let cache = TigerCache::new();
        assert_eq!(cache.document_count(), 0);
        assert!(cache.path.is_none());
    }

    #[test]
    fn test_default() {
        let cache: TigerCache = Default::default();
        assert_eq!(cache.document_count(), 0);
        assert!(cache.path.is_none());
    }

    #[test]
    fn test_open_nonexistent() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("nonexistent.bin");
        
        let cache = TigerCache::open(&file_path).unwrap();
        assert_eq!(cache.document_count(), 0);
        assert_eq!(cache.path.as_ref().unwrap(), &file_path);
    }

    #[test]
    fn test_set_indexed_fields() {
        let mut cache = TigerCache::new();
        let fields = vec!["title".to_string(), "description".to_string()];
        
        cache.set_indexed_fields(fields.clone());
        
        // We can't directly access the indexed_fields, but we can test the functionality
        let mut doc = Document::new("test");
        doc.add_field("title", "Test Title")
           .add_field("description", "Test Description")
           .add_field("irrelevant", "This should not be indexed");
        
        cache.add_document(doc).unwrap();
        
        // Search for a term in the indexed fields
        let results = cache.search("title", None).unwrap();
        assert!(!results.is_empty());
        
        // Search for a term in the non-indexed field
        let results = cache.search("irrelevant", None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_add_document() {
        let mut cache = TigerCache::new();
        let mut doc = Document::new("test");
        doc.add_field("title", "Test Title");
        
        cache.add_document(doc).unwrap();
        assert_eq!(cache.document_count(), 1);
        
        let retrieved = cache.get_document("test").unwrap();
        assert_eq!(retrieved.id, "test");
    }

    #[test]
    fn test_remove_document() {
        let mut cache = TigerCache::new();
        let mut doc = Document::new("test");
        doc.add_field("title", "Test Title");
        
        cache.add_document(doc).unwrap();
        assert_eq!(cache.document_count(), 1);
        
        cache.remove_document("test").unwrap();
        assert_eq!(cache.document_count(), 0);
        assert!(cache.get_document("test").is_none());
    }

    #[test]
    fn test_remove_nonexistent_document() {
        let mut cache = TigerCache::new();
        let result = cache.remove_document("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_document() {
        let mut cache = TigerCache::new();
        let mut doc = Document::new("test");
        doc.add_field("title", "Test Title");
        
        cache.add_document(doc).unwrap();
        
        let retrieved = cache.get_document("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test");
        
        let nonexistent = cache.get_document("nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_document_count() {
        let mut cache = TigerCache::new();
        assert_eq!(cache.document_count(), 0);
        
        let mut doc1 = Document::new("test1");
        doc1.add_field("title", "Test Title 1");
        cache.add_document(doc1).unwrap();
        assert_eq!(cache.document_count(), 1);
        
        let mut doc2 = Document::new("test2");
        doc2.add_field("title", "Test Title 2");
        cache.add_document(doc2).unwrap();
        assert_eq!(cache.document_count(), 2);
        
        cache.remove_document("test1").unwrap();
        assert_eq!(cache.document_count(), 1);
        
        cache.clear();
        assert_eq!(cache.document_count(), 0);
    }

    #[test]
    fn test_search() {
        let mut cache = TigerCache::new();
        
        let mut doc1 = Document::new("test1");
        doc1.add_field("title", "Apple iPhone");
        cache.add_document(doc1).unwrap();
        
        let mut doc2 = Document::new("test2");
        doc2.add_field("title", "Samsung Galaxy");
        cache.add_document(doc2).unwrap();
        
        // Exact match
        let results = cache.search("Apple", None).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].document.id, "test1");
        
        // Fuzzy match
        let results = cache.search("Samsnug", None).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].document.id, "test2");
        
        // With options
        let options = SearchOptions {
            max_distance: 1,
            score_threshold: 0,
            limit: 1,
        };
        
        let results = cache.search("Aple", Some(options)).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].document.id, "test1");
    }

    #[test]
    fn test_commit_without_path() {
        let cache = TigerCache::new();
        let result = cache.commit();
        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_commit() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_index.bin");
        
        let mut cache = TigerCache::new();
        let mut doc = Document::new("test");
        doc.add_field("title", "Test Title");
        cache.add_document(doc).unwrap();
        
        // Save to file
        cache.save_to_file(&file_path).unwrap();
        assert!(file_path.exists());
        
        // Modify and commit
        let mut doc2 = Document::new("test2");
        doc2.add_field("title", "Test Title 2");
        cache.add_document(doc2).unwrap();
        
        cache.commit().unwrap();
        
        // Load from file and verify
        let loaded_cache = TigerCache::open(&file_path).unwrap();
        assert_eq!(loaded_cache.document_count(), 2);
        assert!(loaded_cache.get_document("test").is_some());
        assert!(loaded_cache.get_document("test2").is_some());
    }

    #[test]
    fn test_clear() {
        let mut cache = TigerCache::new();
        
        let mut doc = Document::new("test");
        doc.add_field("title", "Test Title");
        cache.add_document(doc).unwrap();
        
        assert_eq!(cache.document_count(), 1);
        
        cache.clear();
        assert_eq!(cache.document_count(), 0);
        assert!(cache.get_document("test").is_none());
    }
}

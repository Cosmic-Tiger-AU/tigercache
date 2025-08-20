use crate::document::Document;
use crate::error::Result;
use crate::index::Index;
use crate::persistence::{load_from_file, save_to_file};
use crate::search::{SearchOptions, SearchResult};
use std::path::{Path, PathBuf};

/// Main entry point for the Tiger Cache library
///
/// Provides a simple API for creating, searching, and managing an embedded search engine.
#[derive(Debug)]
pub struct TigerCache {
    /// The underlying index
    index: Index,
    
    /// Path to the index file (if loaded from or saved to disk)
    path: Option<PathBuf>,
}

impl TigerCache {
    /// Create a new empty Tiger Cache instance
    pub fn new() -> Self {
        Self {
            index: Index::new(),
            path: None,
        }
    }
    
    /// Open an existing index from a file, or create a new one if the file doesn't exist
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        
        if path_buf.exists() {
            let index = load_from_file(&path_buf)?;
            Ok(Self {
                index,
                path: Some(path_buf),
            })
        } else {
            let mut instance = Self::new();
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
        self.index.add_document(document)
    }
    
    /// Remove a document from the index
    pub fn remove_document(&mut self, doc_id: &str) -> Result<()> {
        self.index.remove_document(doc_id)
    }
    
    /// Get a document by ID
    pub fn get_document(&self, doc_id: &str) -> Option<&Document> {
        self.index.get_document(doc_id)
    }
    
    /// Get the number of documents in the index
    pub fn document_count(&self) -> usize {
        self.index.document_count()
    }
    
    /// Search the index for documents matching the query
    pub fn search(&self, query: &str, options: Option<SearchOptions>) -> Result<Vec<SearchResult>> {
        self.index.search(query, options)
    }
    
    /// Save the index to the file it was opened from
    pub fn commit(&self) -> Result<()> {
        if let Some(path) = &self.path {
            save_to_file(&self.index, path)?;
            Ok(())
        } else {
            Err(crate::error::TigerCacheError::IoError(
                std::io::Error::other(
                    "No file path specified. Use save_to_file instead.",
                ),
            ))
        }
    }
    
    /// Save the index to a specific file
    pub fn save_to_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path_buf = path.as_ref().to_path_buf();
        save_to_file(&self.index, &path_buf)?;
        self.path = Some(path_buf);
        Ok(())
    }
    
    /// Clear the index
    pub fn clear(&mut self) {
        self.index.clear();
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
            score_threshold: 0.0,
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

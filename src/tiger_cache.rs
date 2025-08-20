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
                std::io::Error::new(
                    std::io::ErrorKind::Other,
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


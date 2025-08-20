use std::path::{Path, PathBuf};

use crate::document::Document;
use crate::error::{Result, TigerCacheError};
use crate::index::Index;
use crate::persistence::{load_from_file, save_to_file};
use crate::search::{SearchOptions, SearchResult};
use crate::config::TigerCacheConfig;

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
}

impl TigerCache {
    /// Create a new empty Tiger Cache instance with default configuration
    pub fn new() -> Self {
        Self {
            index: Index::new(),
            path: None,
            config: TigerCacheConfig::default(),
        }
    }
    
    /// Create a new empty Tiger Cache instance with the specified configuration
    pub fn with_config(config: TigerCacheConfig) -> Self {
        Self {
            index: Index::new(),
            path: config.storage.path.clone(),
            config,
        }
    }
    
    /// Open an existing index from a file, or create a new one if the file doesn't exist
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        
        if path_buf.exists() {
            // Try to load the index from the file
            let index = load_from_file(&path_buf)?;
            
            Ok(Self {
                index,
                path: Some(path_buf),
                config: TigerCacheConfig::default(),
            })
        } else {
            // Create a new instance
            let mut instance = Self::new();
            instance.path = Some(path_buf);
            Ok(instance)
        }
    }
    
    /// Open an existing index from a file with the specified configuration
    pub fn open_with_config<P: AsRef<Path>>(path: P, config: TigerCacheConfig) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        
        if path_buf.exists() {
            // Try to load the index from the file
            let index = load_from_file(&path_buf)?;
            
            Ok(Self {
                index,
                path: Some(path_buf),
                config,
            })
        } else {
            // Create a new instance
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
        self.index.add_document(document)
    }
    
    /// Add multiple documents to the index efficiently
    pub fn add_documents_batch(&mut self, documents: Vec<Document>) -> Result<()> {
        self.index.add_documents_batch(documents)
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
    
    /// Search for documents matching the query
    pub fn search(&self, query: &str, options: Option<SearchOptions>) -> Result<Vec<SearchResult>> {
        self.index.search(query, options)
    }
    
    /// Save the index to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Save the index to the file
        save_to_file(&self.index, path.as_ref())?;
        
        // Update the path
        let this = unsafe { &mut *(self as *const _ as *mut Self) };
        this.path = Some(path.as_ref().to_path_buf());
        
        Ok(())
    }
    
    /// Commit changes to disk
    pub fn commit(&self) -> Result<()> {
        // If we have a path, save to it
        if let Some(path) = &self.path {
            save_to_file(&self.index, path)?;
            Ok(())
        } else {
            Err(TigerCacheError::IoError("No path specified for commit".to_string()))
        }
    }
    
    /// Clear the index
    pub fn clear(&mut self) {
        self.index.clear();
    }
    
    /// Flush all caches and storage
    pub fn flush(&self) -> Result<()> {
        // If we have a path, save to it
        if let Some(path) = &self.path {
            save_to_file(&self.index, path)?;
        }
        
        Ok(())
    }
    
    /// Close the TigerCache instance
    pub fn close(&self) -> Result<()> {
        // If we have a path, save to it
        if let Some(path) = &self.path {
            save_to_file(&self.index, path)?;
        }
        
        Ok(())
    }
    
    /// Get the configuration
    pub fn config(&self) -> &TigerCacheConfig {
        &self.config
    }
    
    /// Update the configuration
    pub fn update_config(&mut self, config: TigerCacheConfig) -> Result<()> {
        self.config = config;
        Ok(())
    }
}

impl Default for TigerCache {
    fn default() -> Self {
        Self::new()
    }
}


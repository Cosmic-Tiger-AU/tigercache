use crate::document::Document;
use crate::error::{TigerCacheError, Result};
use crate::trigram::{extract_tokens, generate_trigrams, normalize_text};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// The main index structure that holds documents and search indices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    /// Map of document IDs to documents
    documents: HashMap<String, Document>,
    
    /// Inverted index mapping tokens to document IDs
    inverted_index: HashMap<String, HashSet<String>>,
    
    /// Trigram index mapping trigrams to tokens
    trigram_index: HashMap<String, HashSet<String>>,
    
    /// Fields to be indexed for search
    indexed_fields: Vec<String>,
}

impl Index {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            inverted_index: HashMap::new(),
            trigram_index: HashMap::new(),
            indexed_fields: Vec::new(),
        }
    }
    
    /// Set the fields to be indexed for search
    pub fn set_indexed_fields(&mut self, fields: Vec<String>) -> &mut Self {
        self.indexed_fields = fields;
        self
    }
    
    /// Add a document to the index
    pub fn add_document(&mut self, document: Document) -> Result<()> {
        let doc_id = document.id.clone();
        
        // Extract tokens from indexed fields
        let mut all_tokens = HashSet::new();
        
        if self.indexed_fields.is_empty() {
            // If no specific fields are set, index all text fields
            for text in document.get_all_text_fields() {
                for token in extract_tokens(&text) {
                    all_tokens.insert(token);
                }
            }
        } else {
            // Otherwise, only index the specified fields
            for field_name in &self.indexed_fields {
                if let Some(text) = document.get_text_field(field_name) {
                    for token in extract_tokens(&text) {
                        all_tokens.insert(token);
                    }
                }
            }
        }
        
        // Update inverted index and trigram index
        for token in all_tokens {
            // Add document ID to inverted index for this token
            self.inverted_index
                .entry(token.clone())
                .or_default()
                .insert(doc_id.clone());
            
            // Generate trigrams for the token
            let trigrams = generate_trigrams(&token);
            
            // Add token to trigram index for each trigram
            for trigram in trigrams {
                self.trigram_index
                    .entry(trigram)
                    .or_default()
                    .insert(token.clone());
            }
        }
        
        // Store the document
        self.documents.insert(doc_id, document);
        
        Ok(())
    }
    
    /// Remove a document from the index
    pub fn remove_document(&mut self, doc_id: &str) -> Result<()> {
        if !self.documents.contains_key(doc_id) {
            return Err(TigerCacheError::DocumentNotFound(doc_id.to_string()));
        }
        
        // Remove document ID from inverted index
        for (_, doc_ids) in self.inverted_index.iter_mut() {
            doc_ids.remove(doc_id);
        }
        
        // Remove the document
        self.documents.remove(doc_id);
        
        // Clean up empty entries in inverted index
        self.inverted_index.retain(|_, doc_ids| !doc_ids.is_empty());
        
        // Clean up trigram index (more complex, would need to track token usage)
        // For simplicity, we'll leave this for now and just clean up during reindexing
        
        Ok(())
    }
    
    /// Get a document by ID
    pub fn get_document(&self, doc_id: &str) -> Option<&Document> {
        self.documents.get(doc_id)
    }
    
    /// Get the number of documents in the index
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }
    
    /// Find candidate tokens for a search query using trigram matching
    pub fn find_candidate_tokens(&self, query: &str) -> HashSet<String> {
        let normalized_query = normalize_text(query);
        let query_tokens = extract_tokens(&normalized_query);
        let mut candidate_tokens = HashSet::new();
        
        for query_token in query_tokens {
            let query_trigrams = generate_trigrams(&query_token);
            
            // Find tokens that share at least one trigram with the query token
            for trigram in query_trigrams {
                if let Some(tokens) = self.trigram_index.get(&trigram) {
                    for token in tokens {
                        candidate_tokens.insert(token.clone());
                    }
                }
            }
        }
        
        candidate_tokens
    }
    
    /// Get document IDs containing a specific token
    pub fn get_documents_for_token(&self, token: &str) -> HashSet<String> {
        self.inverted_index
            .get(token)
            .cloned()
            .unwrap_or_else(HashSet::new)
    }
    
    /// Clear the index
    pub fn clear(&mut self) {
        self.documents.clear();
        self.inverted_index.clear();
        self.trigram_index.clear();
    }
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_document(id: &str) -> Document {
        let mut doc = Document::new(id);
        doc.add_field("title", "Test Document")
            .add_field("content", "This is a test document for searching");
        doc
    }
    
    #[test]
    fn test_add_document() {
        let mut index = Index::new();
        let doc = create_test_document("doc1");
        
        assert!(index.add_document(doc).is_ok());
        assert_eq!(index.document_count(), 1);
        
        // Check that tokens were indexed
        assert!(index.inverted_index.contains_key("test"));
        assert!(index.inverted_index.contains_key("document"));
        
        // Check that trigrams were generated
        assert!(index.trigram_index.contains_key("tes"));
        assert!(index.trigram_index.contains_key("est"));
    }
    
    #[test]
    fn test_remove_document() {
        let mut index = Index::new();
        let doc = create_test_document("doc1");
        
        index.add_document(doc).unwrap();
        assert_eq!(index.document_count(), 1);
        
        assert!(index.remove_document("doc1").is_ok());
        assert_eq!(index.document_count(), 0);
        
        // Check that document was removed from inverted index
        for (_, doc_ids) in &index.inverted_index {
            assert!(!doc_ids.contains("doc1"));
        }
    }
    
    #[test]
    fn test_find_candidate_tokens() {
        let mut index = Index::new();
        let doc = create_test_document("doc1");
        
        index.add_document(doc).unwrap();
        
        let candidates = index.find_candidate_tokens("test");
        assert!(candidates.contains("test"));
        
        // Should find similar words with shared trigrams
        let candidates = index.find_candidate_tokens("documant"); // Misspelled
        assert!(candidates.contains("document"));
    }
}

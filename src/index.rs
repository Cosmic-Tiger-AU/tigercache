use crate::document::Document;
use crate::error::{TigerCacheError, Result};
use crate::intern::{StringId, StringInterner};
use crate::trigram::{extract_tokens, generate_trigrams, normalize_text};
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// The main index structure that holds documents and search indices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    /// Map of document IDs to documents
    documents: FxHashMap<StringId, Document>,
    
    /// Inverted index mapping token IDs to document IDs
    inverted_index: FxHashMap<StringId, SmallVec<[StringId; 8]>>,
    
    /// Trigram index mapping trigram IDs to token IDs
    trigram_index: FxHashMap<StringId, SmallVec<[StringId; 4]>>,
    
    /// String interner for memory efficiency
    interner: StringInterner,
    
    /// Fields to be indexed for search
    indexed_fields: Vec<String>,
}

impl Index {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            documents: FxHashMap::default(),
            inverted_index: FxHashMap::default(),
            trigram_index: FxHashMap::default(),
            interner: StringInterner::new(),
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
        let doc_id_str = document.id.clone();
        let doc_id = self.interner.intern(&doc_id_str);
        
        // Extract tokens from indexed fields
        let mut all_tokens = FxHashSet::default();
        
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
            let token_id = self.interner.intern(&token);
            
            // Add document ID to inverted index for this token
            self.inverted_index
                .entry(token_id)
                .or_insert_with(SmallVec::new)
                .push(doc_id);
            
            // Generate trigrams for the token
            let trigrams = generate_trigrams(&token);
            
            // Add token to trigram index for each trigram
            for trigram in trigrams {
                let trigram_id = self.interner.intern(&trigram);
                self.trigram_index
                    .entry(trigram_id)
                    .or_insert_with(SmallVec::new)
                    .push(token_id);
            }
        }
        
        // Store the document
        self.documents.insert(doc_id, document);
        
        Ok(())
    }
    
    /// Add multiple documents to the index efficiently
    pub fn add_documents_batch(&mut self, documents: Vec<Document>) -> Result<()> {
        // Pre-allocate capacity for better performance
        let estimated_tokens = documents.len() * 10; // rough estimate
        self.inverted_index.reserve(estimated_tokens);
        self.trigram_index.reserve(estimated_tokens * 3);
        self.documents.reserve(documents.len());
        
        // Process documents in parallel to extract tokens
        let token_data: Vec<_> = documents
            .par_iter()
            .map(|document| {
                let mut all_tokens = FxHashSet::default();
                
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
                
                (document.id.clone(), all_tokens)
            })
            .collect();
        
        // Now sequentially update the indices to avoid conflicts
        for (doc_id_str, all_tokens) in token_data {
            let doc_id = self.interner.intern(&doc_id_str);
            
            // Update inverted index and trigram index
            for token in all_tokens {
                let token_id = self.interner.intern(&token);
                
                // Add document ID to inverted index for this token
                self.inverted_index
                    .entry(token_id)
                    .or_insert_with(SmallVec::new)
                    .push(doc_id);
                
                // Generate trigrams for the token
                let trigrams = generate_trigrams(&token);
                
                // Add token to trigram index for each trigram
                for trigram in trigrams {
                    let trigram_id = self.interner.intern(&trigram);
                    self.trigram_index
                        .entry(trigram_id)
                        .or_insert_with(SmallVec::new)
                        .push(token_id);
                }
            }
        }
        
        // Store all documents
        for document in documents {
            let doc_id = self.interner.intern(&document.id);
            self.documents.insert(doc_id, document);
        }
        
        Ok(())
    }
    
    /// Remove a document from the index
    pub fn remove_document(&mut self, doc_id: &str) -> Result<()> {
        let doc_id_opt = self.interner.get_id(doc_id);
        let doc_id_interned = match doc_id_opt {
            Some(id) => id,
            None => return Err(TigerCacheError::DocumentNotFound(doc_id.to_string())),
        };
        
        if !self.documents.contains_key(&doc_id_interned) {
            return Err(TigerCacheError::DocumentNotFound(doc_id.to_string()));
        }
        
        // Remove document ID from inverted index
        for (_, doc_ids) in self.inverted_index.iter_mut() {
            doc_ids.retain(|id| *id != doc_id_interned);
        }
        
        // Remove the document
        self.documents.remove(&doc_id_interned);
        
        // Clean up empty entries in inverted index
        self.inverted_index.retain(|_, doc_ids| !doc_ids.is_empty());
        
        // Clean up trigram index (more complex, would need to track token usage)
        // For simplicity, we'll leave this for now and just clean up during reindexing
        
        Ok(())
    }
    
    /// Get a document by ID
    pub fn get_document(&self, doc_id: &str) -> Option<&Document> {
        let doc_id_interned = self.interner.get_id(doc_id)?;
        self.documents.get(&doc_id_interned)
    }
    
    /// Get the number of documents in the index
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }
    
    /// Find candidate tokens for a search query using trigram matching
    pub fn find_candidate_tokens(&self, query: &str) -> FxHashSet<String> {
        let normalized_query = normalize_text(query);
        let query_tokens = extract_tokens(&normalized_query);
        let mut candidate_tokens = FxHashSet::default();
        
        for query_token in query_tokens {
            let query_trigrams = generate_trigrams(&query_token);
            
            // Find tokens that share at least one trigram with the query token
            for trigram in query_trigrams {
                if let Some(trigram_id) = self.interner.get_id(&trigram) {
                    if let Some(token_ids) = self.trigram_index.get(&trigram_id) {
                        for &token_id in token_ids {
                            if let Some(token) = self.interner.get(token_id) {
                                candidate_tokens.insert(token.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        candidate_tokens
    }
    
    /// Get document IDs containing a specific token
    pub fn get_documents_for_token(&self, token: &str) -> Vec<String> {
        if let Some(token_id) = self.interner.get_id(token) {
            if let Some(doc_ids) = self.inverted_index.get(&token_id) {
                return doc_ids.iter()
                    .filter_map(|&doc_id| self.interner.get(doc_id).map(|s| s.to_string()))
                    .collect();
            }
        }
        Vec::new()
    }
    
    /// Clear the index
    pub fn clear(&mut self) {
        self.documents.clear();
        self.inverted_index.clear();
        self.trigram_index.clear();
        self.interner.clear();
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
        let test_id = index.interner.get_id("test");
        let document_id = index.interner.get_id("document");
        assert!(test_id.is_some());
        assert!(document_id.is_some());
        
        // Check that trigrams were generated
        let tes_id = index.interner.get_id("tes");
        let est_id = index.interner.get_id("est");
        assert!(tes_id.is_some());
        assert!(est_id.is_some());
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
        let doc1_id = index.interner.get_id("doc1");
        for (_, doc_ids) in &index.inverted_index {
            if let Some(id) = doc1_id {
                assert!(!doc_ids.contains(&id));
            }
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

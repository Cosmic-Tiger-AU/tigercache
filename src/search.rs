use crate::document::Document;
use crate::error::Result;
use crate::index::Index;
use crate::trigram::extract_tokens;
use levenshtein::levenshtein;
use std::collections::{HashMap, HashSet};

/// Search result with document and score
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matched document
    pub document: Document,
    
    /// Relevance score (higher is better)
    pub score: f64,
}

/// Search configuration options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Maximum Levenshtein distance for fuzzy matching (default: 2)
    pub max_distance: usize,
    
    /// Minimum score threshold for results (default: 0.0)
    pub score_threshold: f64,
    
    /// Maximum number of results to return (default: 100)
    pub limit: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            max_distance: 2,
            score_threshold: 0.0,
            limit: 100,
        }
    }
}

impl Index {
    /// Search the index for documents matching the query
    pub fn search(&self, query: &str, options: Option<SearchOptions>) -> Result<Vec<SearchResult>> {
        let options = options.unwrap_or_default();
        let query_tokens = extract_tokens(query);
        
        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }
        
        // Find candidate tokens using trigram matching
        let mut all_candidate_tokens = HashSet::new();
        for query_token in &query_tokens {
            let candidates = self.find_candidate_tokens(query_token);
            all_candidate_tokens.extend(candidates);
        }
        
        // Filter candidates by Levenshtein distance
        let mut filtered_tokens = HashMap::new();
        for candidate in all_candidate_tokens {
            for query_token in &query_tokens {
                let distance = levenshtein(query_token, &candidate);
                if distance <= options.max_distance {
                    filtered_tokens.insert(candidate.clone(), distance);
                    break;
                }
            }
        }
        
        // Get document IDs for filtered tokens
        let mut document_scores = HashMap::new();
        for (token, distance) in filtered_tokens {
            let doc_ids = self.get_documents_for_token(&token);
            
            // Calculate token score (inverse of distance)
            let token_score = 1.0 / (distance as f64 + 1.0);
            
            // Update document scores
            for doc_id in doc_ids {
                let score = document_scores.entry(doc_id).or_insert(0.0);
                *score += token_score;
            }
        }
        
        // Create search results
        let mut results: Vec<SearchResult> = document_scores
            .iter()
            .filter_map(|(doc_id, score)| {
                if *score < options.score_threshold {
                    return None;
                }
                
                self.get_document(doc_id).map(|doc| SearchResult {
                    document: doc.clone(),
                    score: *score,
                })
            })
            .collect();
        
        // Sort by score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Apply limit
        if results.len() > options.limit {
            results.truncate(options.limit);
        }
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    
    fn create_test_index() -> Index {
        let mut index = Index::new();
        
        let mut doc1 = Document::new("doc1");
        doc1.add_field("title", "Apple iPhone")
            .add_field("description", "The latest smartphone from Apple");
        
        let mut doc2 = Document::new("doc2");
        doc2.add_field("title", "Samsung Galaxy")
            .add_field("description", "Android smartphone with great features");
        
        let mut doc3 = Document::new("doc3");
        doc3.add_field("title", "Google Pixel")
            .add_field("description", "Smartphone with the best camera");
        
        index.add_document(doc1).unwrap();
        index.add_document(doc2).unwrap();
        index.add_document(doc3).unwrap();
        
        index
    }
    
    #[test]
    fn test_search_exact_match() {
        let index = create_test_index();
        
        let results = index.search("Apple", None).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].document.id, "doc1");
    }
    
    #[test]
    fn test_search_fuzzy_match() {
        let index = create_test_index();
        
        // Misspelled "Apple" as "Aple"
        let results = index.search("Aple", None).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].document.id, "doc1");
        
        // Misspelled "Samsung" as "Samsnug"
        let results = index.search("Samsnug", None).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].document.id, "doc2");
    }
    
    #[test]
    fn test_search_with_options() {
        let index = create_test_index();
        
        let options = SearchOptions {
            max_distance: 1, // Stricter fuzzy matching
            score_threshold: 0.5,
            limit: 1,
        };
        
        // This should still match with distance 1
        let results = index.search("Aple", Some(options.clone())).unwrap();
        assert!(!results.is_empty());
        
        // This should not match with max_distance 1 (too many errors)
        let options_strict = SearchOptions {
            max_distance: 0, // No fuzzy matching
            score_threshold: 0.5,
            limit: 1,
        };
        let results = index.search("Appple", Some(options_strict)).unwrap();
        assert!(results.is_empty());
    }
}

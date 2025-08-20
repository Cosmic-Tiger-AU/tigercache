use crate::document::Document;
use crate::error::Result;
use crate::index::Index;
use crate::trigram::extract_tokens;
use levenshtein::levenshtein;
use lru::LruCache;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use std::num::NonZeroUsize;
use std::sync::Mutex;

/// Search result with document and score
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matched document
    pub document: Document,
    
    /// Relevance score (higher is better)
    pub score: f64,
    
    /// Fields that matched the query
    pub matched_fields: Vec<String>,
}

/// Search configuration options
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SearchOptions {
    /// Maximum Levenshtein distance for fuzzy matching (default: 2)
    pub max_distance: usize,
    
    /// Minimum score threshold for results (default: 0.0)
    pub score_threshold: u32, // Changed to u32 for hash compatibility
    
    /// Maximum number of results to return (default: 100)
    pub limit: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            max_distance: 2,
            score_threshold: 0,
            limit: 100,
        }
    }
}

/// Cached search engine with LRU cache
pub struct CachedSearchEngine {
    cache: Mutex<LruCache<(String, SearchOptions), Vec<SearchResult>>>,
}

impl CachedSearchEngine {
    pub fn new(cache_size: usize) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(cache_size).unwrap())),
        }
    }
    
    pub fn search_with_cache(&self, index: &Index, query: &str, options: SearchOptions) -> Result<Vec<SearchResult>> {
        let cache_key = (query.to_string(), options.clone());
        
        // Try to get from cache first
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(cached_results) = cache.get(&cache_key) {
                return Ok(cached_results.clone());
            }
        }
        
        // Not in cache, perform search
        let score_threshold = options.score_threshold as f64 / 1000.0; // Convert back to f64
        let search_opts = SearchOptionsInternal {
            max_distance: options.max_distance,
            score_threshold,
            limit: options.limit,
        };
        
        let results = index.search_internal(query, search_opts)?;
        
        // Cache the results
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(cache_key, results.clone());
        }
        
        Ok(results)
    }
}

/// Internal search options with f64 support
#[derive(Debug, Clone)]
pub(crate) struct SearchOptionsInternal {
    pub max_distance: usize,
    pub score_threshold: f64,
    pub limit: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            max_distance: 2,
            score_threshold: 0, // 0.0 represented as 0
            limit: 100,
        }
    }
}

impl From<SearchOptions> for SearchOptionsInternal {
    fn from(opts: SearchOptions) -> Self {
        Self {
            max_distance: opts.max_distance,
            score_threshold: opts.score_threshold as f64 / 1000.0,
            limit: opts.limit,
        }
    }
}

impl Index {
    /// Search the index for documents matching the query
    pub fn search(&self, query: &str, options: Option<SearchOptions>) -> Result<Vec<SearchResult>> {
        let options = options.unwrap_or_default();
        let internal_options = SearchOptionsInternal::from(options);
        self.search_internal(query, internal_options)
    }
    
    /// Internal search method with f64 options
    pub fn search_internal(&self, query: &str, options: SearchOptionsInternal) -> Result<Vec<SearchResult>> {
        let query_tokens = extract_tokens(query);
        
        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }
        
        // Find candidate tokens with trigram overlap scoring
        let mut candidate_scores = FxHashMap::default();
        for query_token in &query_tokens {
            let query_trigrams = crate::trigram::generate_trigrams(query_token);
            let candidates = self.find_candidate_tokens(query_token);
            
            // Score candidates by trigram overlap
            for candidate in candidates {
                let candidate_trigrams = crate::trigram::generate_trigrams(&candidate);
                let overlap = query_trigrams.intersection(&candidate_trigrams).count();
                let total_trigrams = query_trigrams.len().max(candidate_trigrams.len());
                
                if total_trigrams > 0 {
                    let trigram_score = overlap as f64 / total_trigrams as f64;
                    // Only consider candidates with reasonable trigram overlap
                    if trigram_score >= 0.2 {
                        let distance = levenshtein(query_token, &candidate);
                        if distance <= options.max_distance + 1 {
                            candidate_scores.insert(candidate, (distance, trigram_score));
                        }
                    }
                }
            }
        }
        
        // Filter candidates by Levenshtein distance with parallel processing
        let filtered_tokens: FxHashMap<String, (usize, f64)> = candidate_scores
            .into_par_iter()
            .filter_map(|(candidate, (distance, trigram_score))| {
                if distance <= options.max_distance {
                    Some((candidate, (distance, trigram_score)))
                } else {
                    None
                }
            })
            .collect();
        
        // Get document IDs for filtered tokens with improved scoring
        let mut document_scores = FxHashMap::default();
        for (token, (distance, trigram_score)) in filtered_tokens {
            let doc_ids = self.get_documents_for_token(&token);
            
            // Calculate token score combining distance and trigram overlap
            let distance_score = 1.0 / (distance as f64 + 1.0);
            let combined_score = distance_score * (1.0 + trigram_score);
            
            // Boost exact matches significantly
            let token_score = if distance == 0 { 
                combined_score * 5.0 
            } else { 
                combined_score 
            };
            
            // Update document scores
            for doc_id in doc_ids {
                let score = document_scores.entry(doc_id).or_insert(0.0);
                *score += token_score;
            }
        }
        
        // Create search results with early termination
        let mut results: Vec<SearchResult> = document_scores
            .par_iter()
            .filter_map(|(doc_id, score)| {
                if *score < options.score_threshold {
                    return None;
                }
                
                self.get_document(doc_id).map(|doc| SearchResult {
                    document: doc.clone(),
                    score: *score,
                    matched_fields: Vec::new(),
                })
            })
            .collect();
        
        // Sort by score (descending) with stable sort for consistent results
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.document.id.cmp(&b.document.id))
        });
        
        // Apply limit with early termination
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
            score_threshold: 500,
            limit: 1,
        };
        
        // This should still match with distance 1
        let results = index.search("Aple", Some(options.clone())).unwrap();
        assert!(!results.is_empty());
        
        // This should not match with max_distance 1 (too many errors)
        let options_strict = SearchOptions {
            max_distance: 0, // No fuzzy matching
            score_threshold: 500,
            limit: 1,
        };
        let results = index.search("Appple", Some(options_strict)).unwrap();
        assert!(results.is_empty());
    }
}

use tiger_cache::{Document, TigerCache, SearchOptions, Result};
use tempfile::tempdir;

#[test]
fn test_full_workflow() {
    // Create a new search engine
    let mut tiger_cache = TigerCache::new();
    
    // Add documents
    let mut doc1 = Document::new("doc1");
    doc1.add_field("title", "Apple iPhone")
        .add_field("description", "The latest smartphone from Apple");
    tiger_cache.add_document(doc1).unwrap();
    
    let mut doc2 = Document::new("doc2");
    doc2.add_field("title", "Samsung Galaxy")
        .add_field("description", "Android smartphone with great features");
    tiger_cache.add_document(doc2).unwrap();
    
    // Verify document count
    assert_eq!(tiger_cache.document_count(), 2);
    
    // Search for exact match
    let results = tiger_cache.search("Apple", None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].document.id, "doc1");
    
    // Search with typo
    let results = tiger_cache.search("Samsnug", None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].document.id, "doc2");
    
    // Search with options
    let options = SearchOptions {
        max_distance: 1,
        score_threshold: 0,
        limit: 10,
    };
    
    let results = tiger_cache.search("Aple", Some(options)).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].document.id, "doc1");
    
    // Remove a document
    tiger_cache.remove_document("doc1").unwrap();
    assert_eq!(tiger_cache.document_count(), 1);
    
    // Search again
    let results = tiger_cache.search("Apple", None).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_persistence() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_index.bin");
    
    // Create a new Tiger Cache instance
    let mut tiger_cache = TigerCache::new();
    
    // Add a document
    let mut doc = Document::new("test1");
    doc.add_field("title", "Test Document")
        .add_field("content", "This is a test document for searching");
    tiger_cache.add_document(doc).unwrap();
    
    // Save to file
    tiger_cache.save_to_file(&file_path).unwrap();
    
    // Create a new instance and load from file
    let loaded_cache = TigerCache::open(&file_path).unwrap();
    
    // Verify document count
    assert_eq!(loaded_cache.document_count(), 1);
    
    // Verify document content
    let doc = loaded_cache.get_document("test1").unwrap();
    assert_eq!(doc.get_text_field("title").unwrap(), "Test Document");
    
    // Search
    let results = loaded_cache.search("test", None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].document.id, "test1");
}

#[test]
fn test_empty_and_edge_cases() {
    let mut tiger_cache = TigerCache::new();
    
    // Search empty index
    let results = tiger_cache.search("test", None).unwrap();
    assert!(results.is_empty());
    
    // Empty search query
    let results = tiger_cache.search("", None).unwrap();
    assert!(results.is_empty());
    
    // Add a document
    let mut doc = Document::new("doc1");
    doc.add_field("title", "Test");
    tiger_cache.add_document(doc).unwrap();
    
    // Search for non-existent term
    let results = tiger_cache.search("nonexistent", None).unwrap();
    assert!(results.is_empty());
    
    // Remove non-existent document
    assert!(tiger_cache.remove_document("nonexistent").is_err());
    
    // Clear the index
    tiger_cache.clear();
    assert_eq!(tiger_cache.document_count(), 0);
}

#[test]
fn test_indexed_fields() {
    let mut tiger_cache = TigerCache::new();
    
    // Set specific fields to be indexed
    tiger_cache.set_indexed_fields(vec!["title".to_string()]);
    
    // Add a document with multiple fields
    let mut doc = Document::new("doc1");
    doc.add_field("title", "Indexed Title")
       .add_field("description", "This field is not indexed");
    tiger_cache.add_document(doc).unwrap();
    
    // Search for a term in the indexed field
    let results = tiger_cache.search("Indexed", None).unwrap();
    assert!(!results.is_empty());
    
    // Search for a term in the non-indexed field
    let results = tiger_cache.search("field", None).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_search_options() {
    let mut tiger_cache = TigerCache::new();
    
    // Add documents
    let mut doc1 = Document::new("doc1");
    doc1.add_field("title", "First Document")
        .add_field("score", 10);
    tiger_cache.add_document(doc1).unwrap();
    
    let mut doc2 = Document::new("doc2");
    doc2.add_field("title", "Second Document")
        .add_field("score", 20);
    tiger_cache.add_document(doc2).unwrap();
    
    let mut doc3 = Document::new("doc3");
    doc3.add_field("title", "Third Document")
        .add_field("score", 30);
    tiger_cache.add_document(doc3).unwrap();
    
    // Test with limit
    let options = SearchOptions {
        max_distance: 2,
        score_threshold: 0,
        limit: 1,
    };
    
    let results = tiger_cache.search("Document", Some(options)).unwrap();
    assert_eq!(results.len(), 1);
    
    // Test with higher score threshold
    let options = SearchOptions {
        max_distance: 2,
        score_threshold: 900, // High threshold (0.9 * 1000)
        limit: 10,
    };
    
    let results = tiger_cache.search("Documant", Some(options)).unwrap(); // Typo
    assert!(results.is_empty()); // Should not match due to high threshold
    
    // Test with very strict distance
    let options = SearchOptions {
        max_distance: 0, // No typo tolerance
        score_threshold: 0,
        limit: 10,
    };
    
    let results = tiger_cache.search("Documant", Some(options)).unwrap(); // Typo
    assert!(results.is_empty()); // Should not match due to strict distance
}

#[test]
fn test_large_dataset() {
    let mut tiger_cache = TigerCache::new();
    
    // Add a larger number of documents
    for i in 0..100 {
        let mut doc = Document::new(format!("doc{}", i));
        doc.add_field("title", format!("Document {}", i))
           .add_field("content", format!("This is document number {}", i));
        tiger_cache.add_document(doc).unwrap();
    }
    
    assert_eq!(tiger_cache.document_count(), 100);
    
    // Search for a common term
    let results = tiger_cache.search("document", None).unwrap();
    assert!(!results.is_empty());
    
    // Search with limit
    let options = SearchOptions {
        max_distance: 2,
        score_threshold: 0,
        limit: 10,
    };
    
    let results = tiger_cache.search("document", Some(options)).unwrap();
    assert!(results.len() <= 10);
}

#[test]
fn test_error_handling() {
    let mut tiger_cache = TigerCache::new();
    
    // Test removing a non-existent document
    let result = tiger_cache.remove_document("nonexistent");
    assert!(result.is_err());
    
    // Test committing without a path
    let result = tiger_cache.commit();
    assert!(result.is_err());
    
    // Test loading from a non-existent file
    let result = TigerCache::open("nonexistent_file.bin");
    assert!(result.is_ok()); // Should create a new instance
    
    // Test saving to an invalid path
    let result = tiger_cache.save_to_file("/invalid/path/file.bin");
    assert!(result.is_err());
}

#[test]
fn test_concurrent_operations() {
    use std::thread;
    
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_index.bin");
    
    // Create and save an initial index
    {
        let mut tiger_cache = TigerCache::new();
        let mut doc = Document::new("doc1");
        doc.add_field("title", "Initial Document");
        tiger_cache.add_document(doc).unwrap();
        tiger_cache.save_to_file(&file_path).unwrap();
    }
    
    // Spawn multiple threads to read from the same file
    let mut handles = vec![];
    for _ in 0..5 {
        let file_path = file_path.clone();
        let handle = thread::spawn(move || -> Result<()> {
            let tiger_cache = TigerCache::open(&file_path)?;
            assert_eq!(tiger_cache.document_count(), 1);
            let doc = tiger_cache.get_document("doc1").unwrap();
            assert_eq!(doc.get_text_field("title").unwrap(), "Initial Document");
            Ok(())
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    for handle in handles {
        assert!(handle.join().unwrap().is_ok());
    }
}

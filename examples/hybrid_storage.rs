use std::path::Path;
use bytesize::ByteSize;
use tiger_cache::{
    TigerCache,
    TigerCacheConfig,
    Document,
    SearchOptions,
    StorageType,
    StorageConfig,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory for the example
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("tigercache.db");
    
    println!("Creating TigerCache with hybrid storage at: {:?}", db_path);
    
    // Create a configuration with Sled storage
    let config = TigerCacheConfig::new()
        .with_storage_type(StorageType::Sled)
        .with_storage_path(&db_path)
        .with_cache_size(ByteSize::mib(50))
        .with_max_memory(ByteSize::mib(100))
        .with_collect_metrics(true);
    
    // Create a new TigerCache instance with the configuration
    let mut cache = TigerCache::with_config(config);
    
    // Add some documents
    println!("Adding documents...");
    for i in 0..1000 {
        let doc_id = format!("doc-{}", i);
        let title = format!("Document {}", i);
        let content = format!("This is the content of document {}. It contains some text for searching.", i);
        
        let mut doc = Document::new(&doc_id);
        doc.add_field("title", &title)
           .add_field("content", &content)
           .add_field("category", if i % 3 == 0 { "A" } else if i % 3 == 1 { "B" } else { "C" });
        
        cache.add_document(doc)?;
        
        if i % 100 == 0 {
            println!("Added {} documents", i);
        }
    }
    
    // Commit the changes
    println!("Committing changes...");
    cache.commit()?;
    
    // Print some statistics
    if let Some(memory_stats) = cache.memory_stats() {
        println!("Memory usage: {}", memory_stats.current_usage);
        println!("Memory pressure: {:?}", memory_stats.pressure_level);
    }
    
    if let Some(storage_stats) = cache.storage_stats()? {
        println!("Storage key count: {}", storage_stats.key_count);
        println!("Storage total value size: {} bytes", storage_stats.total_value_size);
        println!("Storage cache hit rate: {:.2}%", storage_stats.cache_hit_rate * 100.0);
    }
    
    // Perform some searches
    println!("\nPerforming searches...");
    
    // First search - should be a cache miss
    let start = std::time::Instant::now();
    let results = cache.search("document", None)?;
    let duration = start.elapsed();
    println!("Search for 'document' found {} results in {:?}", results.len(), duration);
    
    // Second search - should be a cache hit
    let start = std::time::Instant::now();
    let results = cache.search("document", None)?;
    let duration = start.elapsed();
    println!("Repeated search for 'document' found {} results in {:?}", results.len(), duration);
    
    // Search with options
    let options = SearchOptions {
        max_results: Some(5),
        ..Default::default()
    };
    let results = cache.search("content", Some(options))?;
    println!("Search for 'content' with max_results=5 found {} results", results.len());
    
    // Close the cache
    println!("\nClosing cache...");
    cache.close()?;
    
    // Reopen the cache
    println!("Reopening cache...");
    let mut cache = TigerCache::open_with_config(&db_path, TigerCacheConfig::default())?;
    
    // Check if documents were persisted
    println!("Document count after reopening: {}", cache.document_count());
    
    // Get a document
    if let Some(doc) = cache.get_document("doc-42") {
        println!("Retrieved document: {} - {}", doc.id, doc.get_field_text("title").unwrap_or(""));
    }
    
    // Add more documents
    println!("Adding more documents...");
    for i in 1000..1100 {
        let doc_id = format!("doc-{}", i);
        let title = format!("Document {}", i);
        let content = format!("This is the content of document {}. It contains some text for searching.", i);
        
        let mut doc = Document::new(&doc_id);
        doc.add_field("title", &title)
           .add_field("content", &content);
        
        cache.add_document(doc)?;
    }
    
    // Commit the changes
    cache.commit()?;
    
    // Final document count
    println!("Final document count: {}", cache.document_count());
    
    // Close the cache
    cache.close()?;
    
    println!("Example completed successfully!");
    Ok(())
}


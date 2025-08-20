use tiger_cache::{Document, TigerCache, SearchOptions};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Tiger Cache Simple Search Example");
    println!("==================================");

    // Create a new search engine
    let mut tiger_cache = TigerCache::new();
    
    // Set indexed fields (optional)
    tiger_cache.set_indexed_fields(vec!["title".to_string(), "description".to_string()]);
    
    // Add sample documents
    add_sample_documents(&mut tiger_cache)?;
    
    println!("\nAdded {} documents to the index.", tiger_cache.document_count());
    
    // Interactive search loop
    loop {
        print!("\nEnter search query (or 'quit' to exit): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let query = input.trim();
        
        if query.eq_ignore_ascii_case("quit") {
            break;
        }
        
        // Configure search options
        let options = SearchOptions {
            max_distance: 2,
            score_threshold: 0.1,
            limit: 10,
        };
        
        // Perform search
        let results = tiger_cache.search(query, Some(options))?;
        
        if results.is_empty() {
            println!("No results found.");
        } else {
            println!("\nFound {} results:", results.len());
            
            for (i, result) in results.iter().enumerate() {
                println!("\n{}. {} (score: {:.2})", i + 1, result.document.id, result.score);
                
                if let Some(title) = result.document.get_text_field("title") {
                    println!("   Title: {}", title);
                }
                
                if let Some(description) = result.document.get_text_field("description") {
                    println!("   Description: {}", description);
                }
            }
        }
    }
    
    println!("\nGoodbye!");
    
    Ok(())
}

fn add_sample_documents(tiger_cache: &mut TigerCache) -> Result<(), Box<dyn std::error::Error>> {
    // Smartphones
    let mut doc1 = Document::new("iphone-13");
    doc1.add_field("title", "Apple iPhone 13")
        .add_field("category", "Smartphone")
        .add_field("description", "The latest smartphone from Apple with A15 Bionic chip");
    tiger_cache.add_document(doc1)?;
    
    let mut doc2 = Document::new("galaxy-s21");
    doc2.add_field("title", "Samsung Galaxy S21")
        .add_field("category", "Smartphone")
        .add_field("description", "Flagship Android smartphone with excellent camera");
    tiger_cache.add_document(doc2)?;
    
    let mut doc3 = Document::new("pixel-6");
    doc3.add_field("title", "Google Pixel 6")
        .add_field("category", "Smartphone")
        .add_field("description", "Google's smartphone with the best camera and AI features");
    tiger_cache.add_document(doc3)?;
    
    // Laptops
    let mut doc4 = Document::new("macbook-pro");
    doc4.add_field("title", "Apple MacBook Pro")
        .add_field("category", "Laptop")
        .add_field("description", "Powerful laptop for professionals with M1 Pro chip");
    tiger_cache.add_document(doc4)?;
    
    let mut doc5 = Document::new("xps-13");
    doc5.add_field("title", "Dell XPS 13")
        .add_field("category", "Laptop")
        .add_field("description", "Compact and powerful Windows laptop with InfinityEdge display");
    tiger_cache.add_document(doc5)?;
    
    // Tablets
    let mut doc6 = Document::new("ipad-pro");
    doc6.add_field("title", "Apple iPad Pro")
        .add_field("category", "Tablet")
        .add_field("description", "Professional tablet with M1 chip and Liquid Retina XDR display");
    tiger_cache.add_document(doc6)?;
    
    let mut doc7 = Document::new("galaxy-tab-s7");
    doc7.add_field("title", "Samsung Galaxy Tab S7")
        .add_field("category", "Tablet")
        .add_field("description", "Android tablet with S Pen support and 120Hz display");
    tiger_cache.add_document(doc7)?;
    
    // Headphones
    let mut doc8 = Document::new("airpods-pro");
    doc8.add_field("title", "Apple AirPods Pro")
        .add_field("category", "Headphones")
        .add_field("description", "Wireless earbuds with active noise cancellation");
    tiger_cache.add_document(doc8)?;
    
    let mut doc9 = Document::new("sony-wh1000xm4");
    doc9.add_field("title", "Sony WH-1000XM4")
        .add_field("category", "Headphones")
        .add_field("description", "Industry-leading noise cancelling wireless headphones");
    tiger_cache.add_document(doc9)?;
    
    // Smartwatches
    let mut doc10 = Document::new("apple-watch-7");
    doc10.add_field("title", "Apple Watch Series 7")
        .add_field("category", "Smartwatch")
        .add_field("description", "Smartwatch with larger display and fast charging");
    tiger_cache.add_document(doc10)?;
    
    Ok(())
}


use std::time::{Duration, Instant};
use std::fs::File;
use std::io::BufReader;
use serde::{Deserialize, Serialize};
use sysinfo::{System, Pid, ProcessRefreshKind, ProcessesToUpdate};
use tiger_cache::{TigerCache, Document, SearchOptions};

/// Amazon product data structure matching the test dataset
#[derive(Debug, Deserialize, Serialize, Clone)]
struct AmazonProduct {
    parent_asin: String,
    date_first_available: i64,
    title: String,
    description: String,
    store: String,
    details: String,
    main_category: String,
}

/// Benchmark results structure
#[derive(Debug)]
struct BenchmarkResults {
    // Memory metrics (in MB)
    initial_memory_mb: f64,
    peak_memory_mb: f64,
    final_memory_mb: f64,
    memory_growth_mb: f64,
    
    // Timing metrics
    data_load_time: Duration,
    index_creation_time: Duration,
    total_indexing_time: Duration,
    
    // Search performance metrics
    search_benchmarks: Vec<SearchBenchmark>,
    
    // Dataset info
    total_documents: usize,
    indexed_fields: Vec<String>,
}

#[derive(Debug)]
struct SearchBenchmark {
    query: String,
    query_type: String,
    search_time: Duration,
    results_count: usize,
    avg_score: f64,
}

/// Memory tracking utility
struct MemoryTracker {
    system: System,
    pid: Pid,
}

impl MemoryTracker {
    fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let pid = sysinfo::get_current_pid().expect("Failed to get current PID");
        
        Self { system, pid }
    }
    
    fn get_memory_usage_mb(&mut self) -> f64 {
        self.system.refresh_processes_specifics(ProcessesToUpdate::Some(&[self.pid]), ProcessRefreshKind::new().with_memory());
        if let Some(process) = self.system.process(self.pid) {
            process.memory() as f64 / 1024.0 / 1024.0 // Convert to MB
        } else {
            0.0
        }
    }
}

impl BenchmarkResults {
    fn print_report(&self) {
        println!("\n🔥 TigerCache Benchmark Results 🔥");
        println!("================================");
        
        // Dataset Information
        println!("\n📊 Dataset Information:");
        println!("  Total Documents: {}", self.total_documents);
        println!("  Indexed Fields: {:?}", self.indexed_fields);
        
        // Memory Usage
        println!("\n🧠 Memory Usage:");
        println!("  Initial Memory: {:.2} MB", self.initial_memory_mb);
        println!("  Peak Memory: {:.2} MB", self.peak_memory_mb);
        println!("  Final Memory: {:.2} MB", self.final_memory_mb);
        println!("  Memory Growth: {:.2} MB ({:.1}x increase)", 
                 self.memory_growth_mb, 
                 self.final_memory_mb / self.initial_memory_mb);
        
        // Indexing Performance
        println!("\n⚡ Indexing Performance:");
        println!("  Data Loading: {:.2}s", self.data_load_time.as_secs_f64());
        println!("  Index Creation: {:.2}s", self.index_creation_time.as_secs_f64());
        println!("  Total Time: {:.2}s", self.total_indexing_time.as_secs_f64());
        println!("  Documents/sec: {:.0}", 
                 self.total_documents as f64 / self.total_indexing_time.as_secs_f64());
        
        // Search Performance
        println!("\n🔍 Search Performance:");
        for benchmark in &self.search_benchmarks {
            println!("  {} ({}):", benchmark.query, benchmark.query_type);
            println!("    Time: {:.3}ms", benchmark.search_time.as_millis());
            println!("    Results: {} (avg score: {:.3})", 
                     benchmark.results_count, benchmark.avg_score);
        }
        
        // Performance Summary
        let avg_search_time = self.search_benchmarks.iter()
            .map(|b| b.search_time.as_millis())
            .sum::<u128>() as f64 / self.search_benchmarks.len() as f64;
        
        println!("\n📈 Performance Summary:");
        println!("  Avg Search Time: {:.3}ms", avg_search_time);
        println!("  Memory Efficiency: {:.1} docs/MB", 
                 self.total_documents as f64 / self.memory_growth_mb);
        println!("  Search Throughput: ~{:.0} searches/sec", 
                 1000.0 / avg_search_time);
    }
}

fn load_test_data(limit: Option<usize>) -> Result<Vec<AmazonProduct>, Box<dyn std::error::Error>> {
    println!("📂 Loading test data...");
    
    let file = File::open("test_data.json")?;
    let reader = BufReader::new(file);
    
    let mut products = Vec::new();
    let mut line_count = 0;
    
    for line in std::io::BufRead::lines(reader) {
        if let Some(limit) = limit {
            if line_count >= limit {
                break;
            }
        }
        
        let line = line?;
        if !line.trim().is_empty() {
            match serde_json::from_str::<AmazonProduct>(&line) {
                Ok(product) => products.push(product),
                Err(e) => {
                    eprintln!("⚠️  Failed to parse line {}: {} (error: {})", line_count + 1, &line[..50.min(line.len())], e);
                    continue;
                }
            }
        }
        line_count += 1;
        
        // Progress indicator for large datasets
        if line_count % 10000 == 0 {
            println!("  Loaded {} records...", line_count);
        }
    }
    
    println!("✅ Loaded {} products from {} lines", products.len(), line_count);
    Ok(products)
}

fn benchmark_indexing(products: &[AmazonProduct], memory_tracker: &mut MemoryTracker) -> Result<(TigerCache, Duration), Box<dyn std::error::Error>> {
    println!("\n🏗️  Building search index...");
    
    let start_time = Instant::now();
    let mut tiger_cache = TigerCache::new();
    
    // Set up indexed fields - focusing on searchable content
    let indexed_fields = vec![
        "title".to_string(),
        "description".to_string(),
        "store".to_string(),
        "main_category".to_string(),
    ];
    tiger_cache.set_indexed_fields(indexed_fields);
    
    // Add documents with progress tracking
    let total = products.len();
    for (i, product) in products.iter().enumerate() {
        let mut doc = Document::new(&product.parent_asin);
        
        doc.add_field("title", &product.title)
           .add_field("description", &product.description)
           .add_field("store", &product.store)
           .add_field("main_category", &product.main_category)
           .add_field("details", &product.details);
        
        tiger_cache.add_document(doc)?;
        
        // Progress and memory tracking
        if (i + 1) % 5000 == 0 || i + 1 == total {
            let progress = (i + 1) as f64 / total as f64 * 100.0;
            let current_memory = memory_tracker.get_memory_usage_mb();
            println!("  Progress: {:.1}% ({}/{}) - Memory: {:.1} MB", 
                     progress, i + 1, total, current_memory);
        }
    }
    
    let index_time = start_time.elapsed();
    println!("✅ Index created in {:.2}s", index_time.as_secs_f64());
    
    Ok((tiger_cache, index_time))
}

fn benchmark_searches(tiger_cache: &TigerCache) -> Vec<SearchBenchmark> {
    println!("\n🔍 Running search benchmarks...");
    
    let test_queries = vec![
        // Exact matches
        ("iPhone", "exact_product"),
        ("Samsung", "exact_brand"),
        ("Audio CD", "exact_format"),
        
        // Fuzzy matches (with typos)
        ("iPhon", "fuzzy_typo"),
        ("Samsng", "fuzzy_missing_char"),
        ("Musc", "fuzzy_short"),
        
        // Partial matches
        ("Digital Music", "category_match"),
        ("Greatest Hits", "common_phrase"),
        ("Original", "descriptor"),
        
        // Long descriptive searches
        ("smartphone latest Apple", "multi_word"),
        ("Christmas holiday songs", "concept_search"),
        ("jazz piano classical", "genre_search"),
        
        // Edge cases
        ("", "empty_query"),
        ("a", "single_char"),
        ("supercalifragilisticexpialidocious", "nonsense_word"),
    ];
    
    let mut benchmarks = Vec::new();
    
    for (query, query_type) in test_queries {
        print!("  Testing '{}' ({})... ", query, query_type);
        
        let start_time = Instant::now();
        let results = tiger_cache.search(query, None).unwrap_or_default();
        let search_time = start_time.elapsed();
        
        let avg_score = if results.is_empty() { 
            0.0 
        } else { 
            results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64 
        };
        
        println!("{:.3}ms ({} results)", search_time.as_millis(), results.len());
        
        benchmarks.push(SearchBenchmark {
            query: query.to_string(),
            query_type: query_type.to_string(),
            search_time,
            results_count: results.len(),
            avg_score,
        });
        
        // Test with search options for more detailed queries
        if !query.is_empty() && query.len() > 2 {
            let options = SearchOptions {
                max_distance: 2,
                score_threshold: 100, // 0.1 * 1000 as u32
                limit: 10,
            };
            
            let start_time = Instant::now();
            let results = tiger_cache.search(query, Some(options)).unwrap_or_default();
            let search_time = start_time.elapsed();
            
            let avg_score = if results.is_empty() { 
                0.0 
            } else { 
                results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64 
            };
            
            benchmarks.push(SearchBenchmark {
                query: format!("{} (limited)", query),
                query_type: format!("{}_limited", query_type),
                search_time,
                results_count: results.len(),
                avg_score,
            });
        }
    }
    
    benchmarks
}

fn run_benchmark(dataset_limit: Option<usize>) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
    let mut memory_tracker = MemoryTracker::new();
    let initial_memory = memory_tracker.get_memory_usage_mb();
    
    println!("🚀 Starting TigerCache benchmark...");
    println!("Initial memory usage: {:.2} MB", initial_memory);
    
    // Load test data
    let load_start = Instant::now();
    let products = load_test_data(dataset_limit)?;
    let data_load_time = load_start.elapsed();
    
    let after_load_memory = memory_tracker.get_memory_usage_mb();
    println!("Memory after data load: {:.2} MB", after_load_memory);
    
    // Build index
    let (tiger_cache, index_creation_time) = benchmark_indexing(&products, &mut memory_tracker)?;
    let total_indexing_time = load_start.elapsed();
    
    let peak_memory = memory_tracker.get_memory_usage_mb();
    
    // Run search benchmarks
    let search_benchmarks = benchmark_searches(&tiger_cache);
    
    let final_memory = memory_tracker.get_memory_usage_mb();
    
    Ok(BenchmarkResults {
        initial_memory_mb: initial_memory,
        peak_memory_mb: peak_memory,
        final_memory_mb: final_memory,
        memory_growth_mb: final_memory - initial_memory,
        data_load_time,
        index_creation_time,
        total_indexing_time,
        search_benchmarks,
        total_documents: products.len(),
        indexed_fields: vec![
            "title".to_string(),
            "description".to_string(), 
            "store".to_string(),
            "main_category".to_string(),
        ],
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🐅 TigerCache Amazon Dataset Benchmark Tool");
    println!("===========================================\n");
    
    // Parse command line arguments for dataset size limiting
    let args: Vec<String> = std::env::args().collect();
    let dataset_limit = if args.len() > 1 {
        match args[1].parse::<usize>() {
            Ok(limit) => {
                println!("📊 Running benchmark with dataset limited to {} records", limit);
                Some(limit)
            },
            Err(_) => {
                println!("⚠️  Invalid limit argument, using full dataset");
                None
            }
        }
    } else {
        println!("📊 Running benchmark with full dataset");
        println!("💡 Tip: Use 'cargo run --example dataset_test 1000' to limit dataset size");
        None
    };
    
    // Run the main benchmark
    match run_benchmark(dataset_limit) {
        Ok(results) => {
            results.print_report();
            
            // Performance recommendations
            println!("\n💡 Performance Insights:");
            
            if results.memory_growth_mb > 1000.0 {
                println!("  • High memory usage detected. Consider processing in batches for very large datasets.");
            }
            
            let avg_search_time = results.search_benchmarks.iter()
                .map(|b| b.search_time.as_millis())
                .sum::<u128>() as f64 / results.search_benchmarks.len() as f64;
            
            if avg_search_time < 10.0 {
                println!("  • Excellent search performance! ⚡");
            } else if avg_search_time < 50.0 {
                println!("  • Good search performance 👍");
            } else {
                println!("  • Search performance could be optimized for larger datasets");
            }
            
            if results.total_indexing_time.as_secs() < 60 {
                println!("  • Fast indexing performance! 🚀");
            }
            
            println!("\n🎯 Benchmark completed successfully!");
        },
        Err(e) => {
            eprintln!("❌ Benchmark failed: {}", e);
            eprintln!("\n🔧 Troubleshooting:");
            eprintln!("  • Make sure 'test_data.json' exists in the project root");
            eprintln!("  • Verify the JSON format matches the expected schema");
            eprintln!("  • Check available system memory");
            return Err(e);
        }
    }
    
    Ok(())
}

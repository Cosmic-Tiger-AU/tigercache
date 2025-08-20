//! Storage backend comparison benchmark
//!
//! This benchmark compares the performance of different storage backends
//! for TigerCache, including memory usage and query performance.

use std::time::{Duration, Instant};
use std::path::PathBuf;
use bytesize::ByteSize;
use rand::{Rng, SeedableRng};
use rand::distributions::Alphanumeric;
use rand::rngs::StdRng;
use sysinfo::{System, SystemExt, ProcessExt, PidExt};
use uuid::Uuid;

use tiger_cache::storage::{
    StorageConfig,
    StorageType,
    StorageEngine,
    create_storage_engine,
};

/// Benchmark configuration
struct BenchmarkConfig {
    /// Number of documents to insert
    doc_count: usize,
    
    /// Average document size in bytes
    avg_doc_size: usize,
    
    /// Number of queries to run
    query_count: usize,
    
    /// Percentage of queries that are reads (vs writes)
    read_percentage: f64,
    
    /// Random seed for reproducibility
    seed: u64,
    
    /// Storage types to benchmark
    storage_types: Vec<StorageType>,
    
    /// Cache size for each storage type
    cache_size: ByteSize,
    
    /// Whether to run the benchmark in low memory mode
    low_memory_mode: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            doc_count: 10_000,
            avg_doc_size: 1_000,
            query_count: 100_000,
            read_percentage: 0.8,
            seed: 42,
            storage_types: vec![
                StorageType::Memory,
                #[cfg(feature = "sled-storage")]
                StorageType::Sled,
                #[cfg(feature = "redb-storage")]
                StorageType::Redb,
                #[cfg(feature = "rocksdb-storage")]
                StorageType::RocksDB,
            ],
            cache_size: ByteSize::mib(50),
            low_memory_mode: false,
        }
    }
}

/// Benchmark results for a single storage type
struct BenchmarkResult {
    /// Storage type
    storage_type: StorageType,
    
    /// Initial memory usage in bytes
    initial_memory: ByteSize,
    
    /// Peak memory usage in bytes
    peak_memory: ByteSize,
    
    /// Final memory usage in bytes
    final_memory: ByteSize,
    
    /// Insert throughput (docs/sec)
    insert_throughput: f64,
    
    /// Read throughput (queries/sec)
    read_throughput: f64,
    
    /// Write throughput (queries/sec)
    write_throughput: f64,
    
    /// Total benchmark duration
    duration: Duration,
}

/// Run the benchmark
fn run_benchmark(config: BenchmarkConfig) -> Vec<BenchmarkResult> {
    let mut results = Vec::new();
    
    for &storage_type in &config.storage_types {
        println!("Benchmarking storage type: {:?}", storage_type);
        
        // Create a temporary directory for the storage
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
        let storage_path = temp_dir.path().join(format!("tigercache-bench-{}", Uuid::new_v4()));
        
        // Create storage configuration
        let storage_config = if config.low_memory_mode {
            StorageConfig::low_memory()
                .with_storage_type(storage_type)
                .with_path(&storage_path)
                .with_collect_metrics(true)
        } else {
            StorageConfig::new()
                .with_storage_type(storage_type)
                .with_path(&storage_path)
                .with_cache_size(config.cache_size)
                .with_collect_metrics(true)
        };
        
        // Create the storage engine
        let storage = create_storage_engine(storage_config)
            .expect("Failed to create storage engine");
        
        // Initialize random number generator
        let mut rng = StdRng::seed_from_u64(config.seed);
        
        // Initialize system info
        let mut system = System::new_all();
        system.refresh_all();
        
        // Get current process
        let pid = sysinfo::get_current_pid().expect("Failed to get current process ID");
        
        // Measure initial memory usage
        system.refresh_all();
        let initial_memory = if let Some(process) = system.process(pid) {
            ByteSize::b(process.memory() as u64)
        } else {
            ByteSize::b(0)
        };
        
        // Start timing
        let start_time = Instant::now();
        
        // Insert documents
        let insert_start = Instant::now();
        for i in 0..config.doc_count {
            // Generate a random document
            let key = format!("doc:{}", i);
            let value = generate_random_document(&mut rng, config.avg_doc_size);
            
            // Insert the document
            storage.put(key.as_bytes(), &value).expect("Failed to insert document");
            
            // Print progress
            if i % 1000 == 0 {
                println!("Inserted {} documents", i);
            }
        }
        let insert_duration = insert_start.elapsed();
        let insert_throughput = config.doc_count as f64 / insert_duration.as_secs_f64();
        
        // Track peak memory usage
        let mut peak_memory = initial_memory;
        
        // Run queries
        let mut read_count = 0;
        let mut read_duration = Duration::ZERO;
        let mut write_count = 0;
        let mut write_duration = Duration::ZERO;
        
        for _ in 0..config.query_count {
            // Update peak memory usage
            system.refresh_all();
            if let Some(process) = system.process(pid) {
                let current_memory = ByteSize::b(process.memory() as u64);
                if current_memory > peak_memory {
                    peak_memory = current_memory;
                }
            }
            
            // Determine if this is a read or write query
            let is_read = rng.gen_bool(config.read_percentage);
            
            if is_read {
                // Read query
                let doc_id = rng.gen_range(0..config.doc_count);
                let key = format!("doc:{}", doc_id);
                
                let read_start = Instant::now();
                let _ = storage.get(key.as_bytes()).expect("Failed to read document");
                read_duration += read_start.elapsed();
                read_count += 1;
            } else {
                // Write query
                let doc_id = rng.gen_range(0..config.doc_count);
                let key = format!("doc:{}", doc_id);
                let value = generate_random_document(&mut rng, config.avg_doc_size);
                
                let write_start = Instant::now();
                storage.put(key.as_bytes(), &value).expect("Failed to update document");
                write_duration += write_start.elapsed();
                write_count += 1;
            }
        }
        
        // Calculate throughput
        let read_throughput = if read_duration.as_secs_f64() > 0.0 {
            read_count as f64 / read_duration.as_secs_f64()
        } else {
            0.0
        };
        
        let write_throughput = if write_duration.as_secs_f64() > 0.0 {
            write_count as f64 / write_duration.as_secs_f64()
        } else {
            0.0
        };
        
        // Flush and close the storage
        storage.flush().expect("Failed to flush storage");
        storage.close().expect("Failed to close storage");
        
        // Measure final memory usage
        system.refresh_all();
        let final_memory = if let Some(process) = system.process(pid) {
            ByteSize::b(process.memory() as u64)
        } else {
            ByteSize::b(0)
        };
        
        // Record results
        let result = BenchmarkResult {
            storage_type,
            initial_memory,
            peak_memory,
            final_memory,
            insert_throughput,
            read_throughput,
            write_throughput,
            duration: start_time.elapsed(),
        };
        
        results.push(result);
        
        // Print results
        println!("Benchmark results for {:?}:", storage_type);
        println!("  Initial memory: {}", result.initial_memory);
        println!("  Peak memory: {}", result.peak_memory);
        println!("  Final memory: {}", result.final_memory);
        println!("  Insert throughput: {:.2} docs/sec", result.insert_throughput);
        println!("  Read throughput: {:.2} queries/sec", result.read_throughput);
        println!("  Write throughput: {:.2} queries/sec", result.write_throughput);
        println!("  Total duration: {:?}", result.duration);
        println!();
    }
    
    results
}

/// Generate a random document of the specified size
fn generate_random_document(rng: &mut StdRng, avg_size: usize) -> Vec<u8> {
    // Vary the size by ±20%
    let size_variation = (avg_size as f64 * 0.2) as usize;
    let size = avg_size.saturating_sub(size_variation) + rng.gen_range(0..size_variation * 2);
    
    // Generate random bytes
    let random_string: String = (0..size)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect();
    
    random_string.into_bytes()
}

fn main() {
    // Default benchmark configuration
    let config = BenchmarkConfig::default();
    
    // Run the benchmark
    let results = run_benchmark(config);
    
    // Print summary
    println!("Benchmark Summary:");
    println!("{:<10} | {:<15} | {:<15} | {:<15} | {:<20} | {:<20} | {:<20}", 
        "Storage", "Initial Mem", "Peak Mem", "Final Mem", "Insert (docs/s)", "Read (q/s)", "Write (q/s)");
    println!("{:-<10}-+-{:-<15}-+-{:-<15}-+-{:-<15}-+-{:-<20}-+-{:-<20}-+-{:-<20}", 
        "", "", "", "", "", "", "");
    
    for result in &results {
        println!("{:<10?} | {:<15} | {:<15} | {:<15} | {:<20.2} | {:<20.2} | {:<20.2}", 
            result.storage_type,
            result.initial_memory,
            result.peak_memory,
            result.final_memory,
            result.insert_throughput,
            result.read_throughput,
            result.write_throughput);
    }
}


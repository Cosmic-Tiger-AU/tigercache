# TigerCache

An embedded fuzzy search library with hybrid memory/disk storage.

## Features

- **Hybrid Storage**: File-based storage with memory caching, similar to SQLite
- **Memory Management**: Configurable memory limits and intelligent caching
- **Fast Search**: Trigram-based fuzzy search with Levenshtein distance
- **Document Storage**: Store and retrieve documents with multiple fields
- **Configurable**: Multiple storage backends and memory settings

## Storage Backends

TigerCache supports multiple storage backends:

- **Memory**: In-memory storage for testing or small datasets
- **Sled**: Fast embedded database (default)
- **ReDB**: Rust embedded database (optional)
- **RocksDB**: High-performance key-value store (optional)

## Usage

```rust
use tiger_cache::{TigerCache, TigerCacheConfig, Document, StorageType};
use bytesize::ByteSize;

// Create a configuration with Sled storage
let config = TigerCacheConfig::new()
    .with_storage_type(StorageType::Sled)
    .with_storage_path("path/to/database")
    .with_cache_size(ByteSize::mib(50))
    .with_max_memory(ByteSize::mib(100));

// Create a new TigerCache instance
let mut cache = TigerCache::with_config(config);

// Add a document
let mut doc = Document::new("doc1");
doc.add_field("title", "Example Document")
   .add_field("content", "This is an example document for TigerCache");

cache.add_document(doc).unwrap();

// Search for documents
let results = cache.search("example", None).unwrap();
println!("Found {} results", results.len());

// Commit changes to disk
cache.commit().unwrap();

// Close the cache
cache.close().unwrap();
```

## Memory Management

TigerCache includes sophisticated memory management:

- **Memory Pressure Detection**: Monitors memory usage and triggers eviction when needed
- **Multi-tier Caching**: Separate caches for documents, indexes, and query results
- **LRU Eviction**: Least recently used items are evicted first
- **Configurable Limits**: Set memory budgets for different components

## Configuration Presets

TigerCache includes several configuration presets:

- **Default**: Balanced settings for most use cases
- **Development**: Smaller cache sizes for development environments
- **Production**: Larger cache sizes for production environments
- **Low Memory**: Minimal memory usage for resource-constrained environments

```rust
// Use a preset configuration
let config = TigerCacheConfig::low_memory();
let cache = TigerCache::with_config(config);
```

## Performance

TigerCache is designed to handle datasets of 200,000+ documents efficiently, even on lower-end machines. The hybrid storage system allows it to offload data to disk when memory pressure is high, while keeping frequently accessed items in memory for fast access.

## Feature Flags

TigerCache uses feature flags to make storage backends optional:

- `sled-storage`: Enable Sled storage backend (default)
- `redb-storage`: Enable ReDB storage backend
- `rocksdb-storage`: Enable RocksDB storage backend
- `metrics-export`: Enable metrics export via Prometheus
- `all-storage-backends`: Enable all storage backends

## License

TBD


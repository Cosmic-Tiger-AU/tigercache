use std::path::{Path, PathBuf};
use bytesize::ByteSize;
use serde::{Deserialize, Serialize};

/// Storage type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    /// In-memory storage (for testing or small datasets)
    Memory,
    
    /// Sled embedded database
    #[cfg(feature = "sled-storage")]
    Sled,
    
    /// Redb embedded database
    #[cfg(feature = "redb-storage")]
    Redb,
    
    /// RocksDB embedded database
    #[cfg(feature = "rocksdb-storage")]
    RocksDB,
}

impl Default for StorageType {
    fn default() -> Self {
        // Default to the first available storage type in order of preference
        #[cfg(feature = "sled-storage")]
        return StorageType::Sled;
        
        #[cfg(all(not(feature = "sled-storage"), feature = "redb-storage"))]
        return StorageType::Redb;
        
        #[cfg(all(not(feature = "sled-storage"), not(feature = "redb-storage"), feature = "rocksdb-storage"))]
        return StorageType::RocksDB;
        
        #[cfg(all(not(feature = "sled-storage"), not(feature = "redb-storage"), not(feature = "rocksdb-storage")))]
        return StorageType::Memory;
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage type
    pub storage_type: StorageType,
    
    /// Path to the storage directory
    pub path: Option<PathBuf>,
    
    /// Page size in bytes (default: 4KB)
    pub page_size: usize,
    
    /// Cache size in bytes (default: 50MB)
    pub cache_size: ByteSize,
    
    /// Maximum memory usage in bytes (default: 200MB)
    pub max_memory: ByteSize,
    
    /// Whether to create the storage if it doesn't exist
    pub create_if_missing: bool,
    
    /// Whether to use compression
    pub use_compression: bool,
    
    /// Whether to sync writes to disk immediately
    pub sync_writes: bool,
    
    /// Whether to enable metrics collection
    pub collect_metrics: bool,
    
    /// Custom options for specific storage backends
    pub custom_options: Option<serde_json::Value>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            storage_type: StorageType::default(),
            path: None,
            page_size: 4096, // 4KB
            cache_size: ByteSize::mib(50), // 50MB
            max_memory: ByteSize::mib(200), // 200MB
            create_if_missing: true,
            use_compression: true,
            sync_writes: false,
            collect_metrics: false,
            custom_options: None,
        }
    }
}

impl StorageConfig {
    /// Create a new storage configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the storage type
    pub fn with_storage_type(mut self, storage_type: StorageType) -> Self {
        self.storage_type = storage_type;
        self
    }
    
    /// Set the storage path
    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }
    
    /// Set the page size
    pub fn with_page_size(mut self, page_size: usize) -> Self {
        self.page_size = page_size;
        self
    }
    
    /// Set the cache size
    pub fn with_cache_size(mut self, cache_size: ByteSize) -> Self {
        self.cache_size = cache_size;
        self
    }
    
    /// Set the maximum memory usage
    pub fn with_max_memory(mut self, max_memory: ByteSize) -> Self {
        self.max_memory = max_memory;
        self
    }
    
    /// Set whether to create the storage if it doesn't exist
    pub fn with_create_if_missing(mut self, create_if_missing: bool) -> Self {
        self.create_if_missing = create_if_missing;
        self
    }
    
    /// Set whether to use compression
    pub fn with_compression(mut self, use_compression: bool) -> Self {
        self.use_compression = use_compression;
        self
    }
    
    /// Set whether to sync writes to disk immediately
    pub fn with_sync_writes(mut self, sync_writes: bool) -> Self {
        self.sync_writes = sync_writes;
        self
    }
    
    /// Set whether to collect metrics
    pub fn with_collect_metrics(mut self, collect_metrics: bool) -> Self {
        self.collect_metrics = collect_metrics;
        self
    }
    
    /// Set custom options for specific storage backends
    pub fn with_custom_options(mut self, custom_options: serde_json::Value) -> Self {
        self.custom_options = Some(custom_options);
        self
    }
    
    /// Create a development configuration with smaller cache sizes
    pub fn development() -> Self {
        Self {
            cache_size: ByteSize::mib(10), // 10MB
            max_memory: ByteSize::mib(50), // 50MB
            collect_metrics: true,
            ..Default::default()
        }
    }
    
    /// Create a production configuration with larger cache sizes
    pub fn production() -> Self {
        Self {
            cache_size: ByteSize::mib(100), // 100MB
            max_memory: ByteSize::mib(500), // 500MB
            sync_writes: true,
            ..Default::default()
        }
    }
    
    /// Create a low-memory configuration for resource-constrained environments
    pub fn low_memory() -> Self {
        Self {
            cache_size: ByteSize::mib(5), // 5MB
            max_memory: ByteSize::mib(20), // 20MB
            page_size: 1024, // 1KB
            use_compression: true,
            ..Default::default()
        }
    }
}


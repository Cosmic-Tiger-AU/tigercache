use std::path::{Path, PathBuf};
use bytesize::ByteSize;
use serde::{Deserialize, Serialize};

use crate::storage::{StorageConfig, StorageType};

/// TigerCache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TigerCacheConfig {
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// Fields to be indexed for search
    pub indexed_fields: Vec<String>,
    
    /// Maximum Levenshtein distance for fuzzy matching
    pub max_distance: u32,
    
    /// Minimum score threshold for search results
    pub score_threshold: f64,
    
    /// Maximum number of search results
    pub max_results: usize,
    
    /// Whether to enable background operations
    pub enable_background_ops: bool,
    
    /// Whether to collect metrics
    pub collect_metrics: bool,
}

impl Default for TigerCacheConfig {
    fn default() -> Self {
        Self {
            storage: StorageConfig::default(),
            indexed_fields: Vec::new(),
            max_distance: 2,
            score_threshold: 0.0,
            max_results: 100,
            enable_background_ops: true,
            collect_metrics: false,
        }
    }
}

impl TigerCacheConfig {
    /// Create a new TigerCache configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the storage configuration
    pub fn with_storage(mut self, storage: StorageConfig) -> Self {
        self.storage = storage;
        self
    }
    
    /// Set the storage type
    pub fn with_storage_type(mut self, storage_type: StorageType) -> Self {
        self.storage.storage_type = storage_type;
        self
    }
    
    /// Set the storage path
    pub fn with_storage_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.storage.path = Some(path.as_ref().to_path_buf());
        self
    }
    
    /// Set the cache size
    pub fn with_cache_size(mut self, cache_size: ByteSize) -> Self {
        self.storage.cache_size = cache_size;
        self
    }
    
    /// Set the maximum memory usage
    pub fn with_max_memory(mut self, max_memory: ByteSize) -> Self {
        self.storage.max_memory = max_memory;
        self
    }
    
    /// Set the indexed fields
    pub fn with_indexed_fields(mut self, fields: Vec<String>) -> Self {
        self.indexed_fields = fields;
        self
    }
    
    /// Set the maximum Levenshtein distance
    pub fn with_max_distance(mut self, max_distance: u32) -> Self {
        self.max_distance = max_distance;
        self
    }
    
    /// Set the minimum score threshold
    pub fn with_score_threshold(mut self, score_threshold: f64) -> Self {
        self.score_threshold = score_threshold;
        self
    }
    
    /// Set the maximum number of search results
    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = max_results;
        self
    }
    
    /// Set whether to enable background operations
    pub fn with_background_ops(mut self, enable: bool) -> Self {
        self.enable_background_ops = enable;
        self
    }
    
    /// Set whether to collect metrics
    pub fn with_collect_metrics(mut self, collect: bool) -> Self {
        self.collect_metrics = collect;
        self.storage.collect_metrics = collect;
        self
    }
    
    /// Create a development configuration with smaller cache sizes
    pub fn development() -> Self {
        Self {
            storage: StorageConfig::development(),
            indexed_fields: Vec::new(),
            max_distance: 2,
            score_threshold: 0.0,
            max_results: 100,
            enable_background_ops: true,
            collect_metrics: true,
        }
    }
    
    /// Create a production configuration with larger cache sizes
    pub fn production() -> Self {
        Self {
            storage: StorageConfig::production(),
            indexed_fields: Vec::new(),
            max_distance: 2,
            score_threshold: 0.0,
            max_results: 100,
            enable_background_ops: true,
            collect_metrics: false,
        }
    }
    
    /// Create a low-memory configuration for resource-constrained environments
    pub fn low_memory() -> Self {
        Self {
            storage: StorageConfig::low_memory(),
            indexed_fields: Vec::new(),
            max_distance: 1, // Reduce max distance to save memory
            score_threshold: 0.5, // Higher threshold to reduce result set
            max_results: 50, // Fewer results to save memory
            enable_background_ops: false, // Disable background ops to save resources
            collect_metrics: false,
        }
    }
}


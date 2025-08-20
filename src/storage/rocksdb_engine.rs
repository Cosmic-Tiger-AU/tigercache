use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use parking_lot::RwLock;
use rocksdb::{DB, Options, WriteBatch, IteratorMode};

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::config::StorageConfig;
use crate::storage::page::{Page, PageId, PageRef};
use crate::storage::storage_engine::{StorageEngine, StorageTransaction, StorageStats};

// Key prefixes for different data types
const KV_PREFIX: &[u8] = b"kv:";
const PAGE_PREFIX: &[u8] = b"page:";

/// RocksDB storage engine implementation
pub struct RocksDBStorageEngine {
    /// Storage configuration
    config: StorageConfig,
    
    /// RocksDB database
    db: DB,
    
    /// Page cache
    page_cache: RwLock<HashMap<PageId, PageRef>>,
    
    /// Storage statistics
    stats: Arc<Mutex<StorageStats>>,
}

impl RocksDBStorageEngine {
    /// Create a new RocksDB storage engine
    pub fn new(config: StorageConfig) -> StorageResult<Self> {
        // Ensure we have a path
        let path = config.path.clone().ok_or_else(|| {
            StorageError::ConfigError("RocksDB storage requires a path".to_string())
        })?;
        
        // Create RocksDB options
        let mut opts = Options::default();
        opts.create_if_missing(config.create_if_missing);
        
        // Set compression if enabled
        if config.use_compression {
            opts.set_compression_type(rocksdb::DBCompressionType::Snappy);
        }
        
        // Open the database
        let db = DB::open(&opts, path)
            .map_err(|e| StorageError::Other(format!("Failed to open RocksDB: {}", e)))?;
        
        Ok(Self {
            config,
            db,
            page_cache: RwLock::new(HashMap::new()),
            stats: Arc::new(Mutex::new(StorageStats {
                key_count: 0,
                total_value_size: 0,
                page_count: 0,
                dirty_page_count: 0,
                cache_hit_rate: 0.0,
                read_count: 0,
                write_count: 0,
                custom_stats: HashMap::new(),
            })),
        })
    }
    
    /// Create a key with the KV prefix
    fn kv_key(key: &[u8]) -> Vec<u8> {
        let mut prefixed_key = Vec::with_capacity(KV_PREFIX.len() + key.len());
        prefixed_key.extend_from_slice(KV_PREFIX);
        prefixed_key.extend_from_slice(key);
        prefixed_key
    }
    
    /// Create a key with the page prefix
    fn page_key(page_id: PageId) -> Vec<u8> {
        let mut prefixed_key = Vec::with_capacity(PAGE_PREFIX.len() + 8);
        prefixed_key.extend_from_slice(PAGE_PREFIX);
        prefixed_key.extend_from_slice(&page_id.to_be_bytes());
        prefixed_key
    }
}

impl StorageEngine for RocksDBStorageEngine {
    fn config(&self) -> &StorageConfig {
        &self.config
    }
    
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let prefixed_key = Self::kv_key(key);
        
        // Get the value from RocksDB
        let result = self.db.get(&prefixed_key)
            .map_err(|e| StorageError::Other(format!("Failed to get value from RocksDB: {}", e)))?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        Ok(result)
    }
    
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        let prefixed_key = Self::kv_key(key);
        
        // Check if the key exists
        let old_value = self.db.get(&prefixed_key)
            .map_err(|e| StorageError::Other(format!("Failed to get value from RocksDB: {}", e)))?;
        let is_new = old_value.is_none();
        let old_size = old_value.map(|v| v.len()).unwrap_or(0);
        
        // Put the value
        self.db.put(&prefixed_key, value)
            .map_err(|e| StorageError::Other(format!("Failed to put value to RocksDB: {}", e)))?;
        
        // Sync writes if configured
        if self.config.sync_writes {
            self.db.flush()
                .map_err(|e| StorageError::Other(format!("Failed to flush RocksDB: {}", e)))?;
        }
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.write_count += 1;
            if is_new {
                stats.key_count += 1;
            }
            stats.total_value_size = stats.total_value_size - old_size + value.len();
        }
        
        Ok(())
    }
    
    fn delete(&self, key: &[u8]) -> StorageResult<()> {
        let prefixed_key = Self::kv_key(key);
        
        // Check if the key exists
        let old_value = self.db.get(&prefixed_key)
            .map_err(|e| StorageError::Other(format!("Failed to get value from RocksDB: {}", e)))?;
        let old_size = old_value.map(|v| v.len()).unwrap_or(0);
        
        // Delete the key
        self.db.delete(&prefixed_key)
            .map_err(|e| StorageError::Other(format!("Failed to delete key from RocksDB: {}", e)))?;
        
        // Sync writes if configured
        if self.config.sync_writes {
            self.db.flush()
                .map_err(|e| StorageError::Other(format!("Failed to flush RocksDB: {}", e)))?;
        }
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.write_count += 1;
            if old_value.is_some() {
                stats.key_count -= 1;
                stats.total_value_size -= old_size;
            }
        }
        
        Ok(())
    }
    
    fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let prefixed_key = Self::kv_key(key);
        
        // Check if the key exists
        let exists = self.db.get(&prefixed_key)
            .map_err(|e| StorageError::Other(format!("Failed to get value from RocksDB: {}", e)))?
            .is_some();
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        Ok(exists)
    }
    
    fn begin_transaction(&self) -> StorageResult<Box<dyn StorageTransaction + '_>> {
        // Create a new RocksDB transaction
        Ok(Box::new(RocksDBTransaction {
            batch: WriteBatch::default(),
            engine: self,
            committed: false,
        }))
    }
    
    fn get_page(&self, page_id: PageId) -> StorageResult<Option<PageRef>> {
        // First check the cache
        let page_cache = self.page_cache.read();
        if let Some(page) = page_cache.get(&page_id) {
            // Update stats
            if let Ok(mut stats) = self.stats.lock() {
                stats.read_count += 1;
            }
            
            return Ok(Some(page.clone()));
        }
        
        // Get the page from RocksDB
        let page_key = Self::page_key(page_id);
        let page_data = self.db.get(&page_key)
            .map_err(|e| StorageError::Other(format!("Failed to get page from RocksDB: {}", e)))?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        // If the page exists, deserialize it and add to cache
        if let Some(page_data) = page_data {
            let page: Page = bincode::deserialize(&page_data)
                .map_err(|e| StorageError::Other(format!("Failed to deserialize page: {}", e)))?;
            
            let page_ref = Arc::new(RwLock::new(page));
            
            // Add to cache
            let mut page_cache = self.page_cache.write();
            page_cache.insert(page_id, page_ref.clone());
            
            Ok(Some(page_ref))
        } else {
            Ok(None)
        }
    }
    
    fn put_page(&self, page: Page) -> StorageResult<()> {
        let page_id = page.id;
        let page_ref = Arc::new(RwLock::new(page));
        
        // Add to cache
        let mut page_cache = self.page_cache.write();
        let is_new = !page_cache.contains_key(&page_id);
        page_cache.insert(page_id, page_ref.clone());
        
        // Serialize the page
        let page_data = bincode::serialize(&*page_ref.read())
            .map_err(|e| StorageError::Other(format!("Failed to serialize page: {}", e)))?;
        
        // Put the page
        let page_key = Self::page_key(page_id);
        self.db.put(&page_key, &page_data)
            .map_err(|e| StorageError::Other(format!("Failed to put page to RocksDB: {}", e)))?;
        
        // Sync writes if configured
        if self.config.sync_writes {
            self.db.flush()
                .map_err(|e| StorageError::Other(format!("Failed to flush RocksDB: {}", e)))?;
        }
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.write_count += 1;
            if is_new {
                stats.page_count += 1;
            }
            // Count dirty pages
            stats.dirty_page_count = page_cache.values()
                .filter(|p| p.read().is_dirty())
                .count();
        }
        
        Ok(())
    }
    
    fn flush(&self) -> StorageResult<()> {
        self.db.flush()
            .map_err(|e| StorageError::Other(format!("Failed to flush RocksDB: {}", e)))?;
        
        Ok(())
    }
    
    fn close(&self) -> StorageResult<()> {
        // RocksDB automatically closes on drop
        Ok(())
    }
    
    fn stats(&self) -> StorageResult<StorageStats> {
        if let Ok(stats) = self.stats.lock() {
            Ok(stats.clone())
        } else {
            Err(StorageError::Other("Failed to get storage stats".to_string()))
        }
    }
    
    fn path(&self) -> Option<&PathBuf> {
        self.config.path.as_ref()
    }
    
    fn storage_type(&self) -> &'static str {
        "rocksdb"
    }
}

/// RocksDB transaction implementation
struct RocksDBTransaction<'a> {
    batch: WriteBatch,
    engine: &'a RocksDBStorageEngine,
    committed: bool,
}

impl<'a> StorageTransaction for RocksDBTransaction<'a> {
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        // RocksDB WriteBatch doesn't support reads, so we read from the engine
        self.engine.get(key)
    }
    
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        if self.committed {
            return Err(StorageError::TransactionError("Transaction already committed".to_string()));
        }
        
        let prefixed_key = RocksDBStorageEngine::kv_key(key);
        self.batch.put(&prefixed_key, value);
        
        Ok(())
    }
    
    fn delete(&self, key: &[u8]) -> StorageResult<()> {
        if self.committed {
            return Err(StorageError::TransactionError("Transaction already committed".to_string()));
        }
        
        let prefixed_key = RocksDBStorageEngine::kv_key(key);
        self.batch.delete(&prefixed_key);
        
        Ok(())
    }
    
    fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        // RocksDB WriteBatch doesn't support reads, so we read from the engine
        self.engine.exists(key)
    }
    
    fn commit(mut self: Box<Self>) -> StorageResult<()> {
        if self.committed {
            return Err(StorageError::TransactionError("Transaction already committed".to_string()));
        }
        
        // Write the batch
        self.engine.db.write(&self.batch)
            .map_err(|e| StorageError::Other(format!("Failed to commit RocksDB transaction: {}", e)))?;
        
        // Sync writes if configured
        if self.engine.config.sync_writes {
            self.engine.db.flush()
                .map_err(|e| StorageError::Other(format!("Failed to flush RocksDB: {}", e)))?;
        }
        
        self.committed = true;
        Ok(())
    }
    
    fn abort(mut self: Box<Self>) -> StorageResult<()> {
        self.committed = true; // Mark as committed to prevent double-abort
        Ok(())
    }
}

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use parking_lot::RwLock;

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::config::StorageConfig;
use crate::storage::page::{Page, PageId, PageRef};

/// Storage transaction trait
pub trait StorageTransaction: Send + Sync {
    /// Get a value by key
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>>;
    
    /// Put a key-value pair
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()>;
    
    /// Delete a key
    fn delete(&self, key: &[u8]) -> StorageResult<()>;
    
    /// Check if a key exists
    fn exists(&self, key: &[u8]) -> StorageResult<bool>;
    
    /// Commit the transaction
    fn commit(self: Box<Self>) -> StorageResult<()>;
    
    /// Abort the transaction
    fn abort(self: Box<Self>) -> StorageResult<()>;
}

/// Storage engine trait
pub trait StorageEngine: Send + Sync {
    /// Get the storage configuration
    fn config(&self) -> &StorageConfig;
    
    /// Get a value by key
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>>;
    
    /// Put a key-value pair
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()>;
    
    /// Delete a key
    fn delete(&self, key: &[u8]) -> StorageResult<()>;
    
    /// Check if a key exists
    fn exists(&self, key: &[u8]) -> StorageResult<bool>;
    
    /// Begin a transaction
    fn begin_transaction(&self) -> StorageResult<Box<dyn StorageTransaction + '_>>;
    
    /// Get a page by ID
    fn get_page(&self, page_id: PageId) -> StorageResult<Option<PageRef>>;
    
    /// Put a page
    fn put_page(&self, page: Page) -> StorageResult<()>;
    
    /// Flush all dirty pages to disk
    fn flush(&self) -> StorageResult<()>;
    
    /// Close the storage engine
    fn close(&self) -> StorageResult<()>;
    
    /// Get storage statistics
    fn stats(&self) -> StorageResult<StorageStats>;
    
    /// Get the storage path
    fn path(&self) -> Option<&PathBuf>;
    
    /// Get the storage type
    fn storage_type(&self) -> &'static str;
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Number of keys in the storage
    pub key_count: usize,
    
    /// Total size of all values in bytes
    pub total_value_size: usize,
    
    /// Number of pages in the storage
    pub page_count: usize,
    
    /// Number of dirty pages
    pub dirty_page_count: usize,
    
    /// Cache hit rate (0.0 - 1.0)
    pub cache_hit_rate: f64,
    
    /// Number of reads
    pub read_count: u64,
    
    /// Number of writes
    pub write_count: u64,
    
    /// Custom statistics for specific storage backends
    pub custom_stats: HashMap<String, String>,
}

/// In-memory storage engine implementation for testing or small datasets
pub struct MemoryStorageEngine {
    /// Storage configuration
    config: StorageConfig,
    
    /// In-memory key-value store
    data: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
    
    /// In-memory page store
    pages: RwLock<HashMap<PageId, PageRef>>,
    
    /// Storage statistics
    stats: Arc<Mutex<StorageStats>>,
}

impl MemoryStorageEngine {
    /// Create a new in-memory storage engine
    pub fn new(config: StorageConfig) -> StorageResult<Self> {
        Ok(Self {
            config,
            data: RwLock::new(HashMap::new()),
            pages: RwLock::new(HashMap::new()),
            stats: Arc::new(Mutex::new(StorageStats {
                key_count: 0,
                total_value_size: 0,
                page_count: 0,
                dirty_page_count: 0,
                cache_hit_rate: 1.0, // Always hit in memory
                read_count: 0,
                write_count: 0,
                custom_stats: HashMap::new(),
            })),
        })
    }
}

impl StorageEngine for MemoryStorageEngine {
    fn config(&self) -> &StorageConfig {
        &self.config
    }
    
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let data = self.data.read();
        let result = data.get(key).cloned();
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        Ok(result)
    }
    
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        let mut data = self.data.write();
        let is_new = !data.contains_key(key);
        let old_size = if is_new { 0 } else { data.get(key).map(|v| v.len()).unwrap_or(0) };
        
        data.insert(key.to_vec(), value.to_vec());
        
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
        let mut data = self.data.write();
        let old_size = data.get(key).map(|v| v.len()).unwrap_or(0);
        let removed = data.remove(key);
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.write_count += 1;
            if removed.is_some() {
                stats.key_count -= 1;
                stats.total_value_size -= old_size;
            }
        }
        
        Ok(())
    }
    
    fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let data = self.data.read();
        let result = data.contains_key(key);
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        Ok(result)
    }
    
    fn begin_transaction(&self) -> StorageResult<Box<dyn StorageTransaction + '_>> {
        // For in-memory, we'll use a simple transaction that just clones the data
        Ok(Box::new(MemoryTransaction {
            engine: self,
            changes: HashMap::new(),
            committed: false,
        }))
    }
    
    fn get_page(&self, page_id: PageId) -> StorageResult<Option<PageRef>> {
        let pages = self.pages.read();
        let page = pages.get(&page_id).cloned();
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        Ok(page)
    }
    
    fn put_page(&self, page: Page) -> StorageResult<()> {
        let page_id = page.id;
        let page_ref = Arc::new(RwLock::new(page));
        
        let mut pages = self.pages.write();
        let is_new = !pages.contains_key(&page_id);
        pages.insert(page_id, page_ref);
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.write_count += 1;
            if is_new {
                stats.page_count += 1;
            }
            // Count dirty pages
            stats.dirty_page_count = pages.values()
                .filter(|p| p.read().is_dirty())
                .count();
        }
        
        Ok(())
    }
    
    fn flush(&self) -> StorageResult<()> {
        // For in-memory, flush does nothing
        Ok(())
    }
    
    fn close(&self) -> StorageResult<()> {
        // For in-memory, close does nothing
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
        "memory"
    }
}

/// In-memory transaction implementation
struct MemoryTransaction<'a> {
    engine: &'a MemoryStorageEngine,
    changes: HashMap<Vec<u8>, Option<Vec<u8>>>,
    committed: bool,
}

impl<'a> StorageTransaction for MemoryTransaction<'a> {
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        // First check the transaction changes
        if let Some(value) = self.changes.get(key) {
            return Ok(value.clone());
        }
        
        // Then check the engine
        self.engine.get(key)
    }
    
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        let mut changes = self.changes.clone();
        changes.insert(key.to_vec(), Some(value.to_vec()));
        Ok(())
    }
    
    fn delete(&self, key: &[u8]) -> StorageResult<()> {
        let mut changes = self.changes.clone();
        changes.insert(key.to_vec(), None);
        Ok(())
    }
    
    fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        // First check the transaction changes
        if let Some(value) = self.changes.get(key) {
            return Ok(value.is_some());
        }
        
        // Then check the engine
        self.engine.exists(key)
    }
    
    fn commit(mut self: Box<Self>) -> StorageResult<()> {
        if self.committed {
            return Err(StorageError::TransactionError("Transaction already committed".to_string()));
        }
        
        // Apply all changes to the engine
        for (key, value_opt) in &self.changes {
            match value_opt {
                Some(value) => self.engine.put(key, value)?,
                None => self.engine.delete(key)?,
            }
        }
        
        self.committed = true;
        Ok(())
    }
    
    fn abort(mut self: Box<Self>) -> StorageResult<()> {
        self.committed = true; // Mark as committed to prevent double-abort
        Ok(())
    }
}

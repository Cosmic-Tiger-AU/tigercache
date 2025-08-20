#![cfg(feature = "sled-storage")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use parking_lot::RwLock;

use sled::{Db, IVec, Tree};

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::config::StorageConfig;
use crate::storage::page::{Page, PageId, PageRef};
use crate::storage::storage_engine::{StorageEngine, StorageTransaction, StorageStats};

/// Sled storage engine implementation
pub struct SledStorageEngine {
    /// Storage configuration
    config: StorageConfig,
    
    /// Sled database instance
    db: Db,
    
    /// Main key-value tree
    main_tree: Tree,
    
    /// Pages tree
    pages_tree: Tree,
    
    /// In-memory page cache
    page_cache: RwLock<HashMap<PageId, PageRef>>,
    
    /// Storage statistics
    stats: Arc<Mutex<StorageStats>>,
}

impl SledStorageEngine {
    /// Create a new Sled storage engine
    pub fn new(config: StorageConfig) -> StorageResult<Self> {
        let path = config.path.clone().ok_or_else(|| {
            StorageError::ConfigurationError("Storage path is required for Sled engine".to_string())
        })?;
        
        // Configure Sled
        let mut sled_config = sled::Config::new()
            .path(&path)
            .cache_capacity(config.cache_size.as_u64() as usize)
            .mode(if config.sync_writes {
                sled::Mode::HighThroughput
            } else {
                sled::Mode::LowSpace
            });
        
        // Apply compression if enabled
        if config.use_compression {
            sled_config = sled_config.use_compression(true);
        }
        
        // Open the database
        let db = sled_config.open()?;
        
        // Open or create trees
        let main_tree = db.open_tree("main")?;
        let pages_tree = db.open_tree("pages")?;
        
        Ok(Self {
            config,
            db,
            main_tree,
            pages_tree,
            page_cache: RwLock::new(HashMap::new()),
            stats: Arc::new(Mutex::new(StorageStats {
                key_count: 0, // Will be updated in the background
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
    
    /// Update storage statistics
    fn update_stats(&self) -> StorageResult<()> {
        if let Ok(mut stats) = self.stats.lock() {
            // Count keys in main tree
            stats.key_count = self.main_tree.len() as usize;
            
            // Count pages
            stats.page_count = self.pages_tree.len() as usize;
            
            // Count dirty pages
            let page_cache = self.page_cache.read();
            stats.dirty_page_count = page_cache.values()
                .filter(|p| p.read().is_dirty())
                .count();
            
            // Get Sled stats
            if let Some(sled_stats) = self.db.statistics() {
                stats.custom_stats.insert("sled_cache_hits".to_string(), sled_stats.tree_cache_hits.to_string());
                stats.custom_stats.insert("sled_cache_misses".to_string(), sled_stats.tree_cache_misses.to_string());
                
                // Calculate cache hit rate
                let hits = sled_stats.tree_cache_hits as f64;
                let misses = sled_stats.tree_cache_misses as f64;
                let total = hits + misses;
                
                if total > 0.0 {
                    stats.cache_hit_rate = hits / total;
                }
            }
        }
        
        Ok(())
    }
}

impl StorageEngine for SledStorageEngine {
    fn config(&self) -> &StorageConfig {
        &self.config
    }
    
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let result = self.main_tree.get(key)?
            .map(|ivec| ivec.to_vec());
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        Ok(result)
    }
    
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        self.main_tree.insert(key, value)?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.write_count += 1;
        }
        
        Ok(())
    }
    
    fn delete(&self, key: &[u8]) -> StorageResult<()> {
        self.main_tree.remove(key)?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.write_count += 1;
        }
        
        Ok(())
    }
    
    fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let result = self.main_tree.contains_key(key)?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        Ok(result)
    }
    
    fn begin_transaction(&self) -> StorageResult<Box<dyn StorageTransaction>> {
        Ok(Box::new(SledTransaction {
            main_tree: self.main_tree.clone(),
            changes: HashMap::new(),
            committed: false,
        }))
    }
    
    fn get_page(&self, page_id: PageId) -> StorageResult<Option<PageRef>> {
        // First check the cache
        let cache = self.page_cache.read();
        if let Some(page_ref) = cache.get(&page_id) {
            // Update access time
            page_ref.write().touch();
            
            // Update stats
            if let Ok(mut stats) = self.stats.lock() {
                stats.read_count += 1;
            }
            
            return Ok(Some(page_ref.clone()));
        }
        drop(cache);
        
        // Not in cache, try to load from disk
        let page_key = page_id.to_be_bytes();
        if let Some(page_data) = self.pages_tree.get(&page_key)? {
            // Deserialize the page
            let page: Page = bincode::decode_from_slice(&page_data, bincode::config::standard())?
                .0;
            
            // Create a new page reference
            let page_ref = Arc::new(RwLock::new(page));
            
            // Add to cache
            let mut cache = self.page_cache.write();
            cache.insert(page_id, page_ref.clone());
            
            // Update stats
            if let Ok(mut stats) = self.stats.lock() {
                stats.read_count += 1;
            }
            
            Ok(Some(page_ref))
        } else {
            Ok(None)
        }
    }
    
    fn put_page(&self, page: Page) -> StorageResult<()> {
        let page_id = page.id;
        let page_ref = Arc::new(RwLock::new(page));
        
        // Add to cache
        let mut cache = self.page_cache.write();
        cache.insert(page_id, page_ref.clone());
        drop(cache);
        
        // If the page is dirty, write it to disk
        if page_ref.read().is_dirty() {
            let page_key = page_id.to_be_bytes();
            let page_data = bincode::encode_to_vec(&*page_ref.read(), bincode::config::standard())?;
            self.pages_tree.insert(&page_key, page_data)?;
            
            // Mark the page as clean
            page_ref.write().mark_clean();
        }
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.write_count += 1;
        }
        
        Ok(())
    }
    
    fn flush(&self) -> StorageResult<()> {
        // Flush all dirty pages to disk
        let cache = self.page_cache.read();
        for (page_id, page_ref) in cache.iter() {
            if page_ref.read().is_dirty() {
                let page_key = page_id.to_be_bytes();
                let page_data = bincode::encode_to_vec(&*page_ref.read(), bincode::config::standard())?;
                self.pages_tree.insert(&page_key, page_data)?;
                
                // Mark the page as clean
                page_ref.write().mark_clean();
            }
        }
        
        // Flush the database
        self.db.flush()?;
        
        // Update stats
        self.update_stats()?;
        
        Ok(())
    }
    
    fn close(&self) -> StorageResult<()> {
        // Flush all dirty pages
        self.flush()?;
        
        // Close the database
        self.db.flush()?;
        
        Ok(())
    }
    
    fn stats(&self) -> StorageResult<StorageStats> {
        // Update stats before returning
        self.update_stats()?;
        
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
        "sled"
    }
}

/// Sled transaction implementation
struct SledTransaction {
    main_tree: Tree,
    changes: HashMap<Vec<u8>, Option<IVec>>,
    committed: bool,
}

impl StorageTransaction for SledTransaction {
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        // First check the transaction changes
        if let Some(value_opt) = self.changes.get(key) {
            return Ok(value_opt.as_ref().map(|ivec| ivec.to_vec()));
        }
        
        // Then check the tree
        let result = self.main_tree.get(key)?
            .map(|ivec| ivec.to_vec());
        
        Ok(result)
    }
    
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        let mut changes = self.changes.clone();
        changes.insert(key.to_vec(), Some(IVec::from(value)));
        Ok(())
    }
    
    fn delete(&self, key: &[u8]) -> StorageResult<()> {
        let mut changes = self.changes.clone();
        changes.insert(key.to_vec(), None);
        Ok(())
    }
    
    fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        // First check the transaction changes
        if let Some(value_opt) = self.changes.get(key) {
            return Ok(value_opt.is_some());
        }
        
        // Then check the tree
        let result = self.main_tree.contains_key(key)?;
        
        Ok(result)
    }
    
    fn commit(mut self: Box<Self>) -> StorageResult<()> {
        if self.committed {
            return Err(StorageError::TransactionError("Transaction already committed".to_string()));
        }
        
        // Apply all changes to the tree
        for (key, value_opt) in &self.changes {
            match value_opt {
                Some(value) => self.main_tree.insert(key, value.clone())?,
                None => self.main_tree.remove(key)?,
            };
        }
        
        self.committed = true;
        Ok(())
    }
    
    fn abort(mut self: Box<Self>) -> StorageResult<()> {
        self.committed = true; // Mark as committed to prevent double-abort
        Ok(())
    }
}


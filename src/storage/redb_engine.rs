use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use parking_lot::RwLock;
use redb::{Database, ReadableTable, TableDefinition};

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::config::StorageConfig;
use crate::storage::page::{Page, PageId, PageRef};
use crate::storage::storage_engine::{StorageEngine, StorageTransaction, StorageStats};

// Table definitions
const KV_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("kv");
const PAGE_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("pages");

/// ReDB storage engine implementation
pub struct RedbStorageEngine {
    /// Storage configuration
    config: StorageConfig,
    
    /// ReDB database
    db: Database,
    
    /// Page cache
    page_cache: RwLock<HashMap<PageId, PageRef>>,
    
    /// Storage statistics
    stats: Arc<Mutex<StorageStats>>,
}

impl RedbStorageEngine {
    /// Create a new ReDB storage engine
    pub fn new(config: StorageConfig) -> StorageResult<Self> {
        // Ensure we have a path
        let path = config.path.clone().ok_or_else(|| {
            StorageError::ConfigError("ReDB storage requires a path".to_string())
        })?;
        
        // Create the database
        let db = Database::create(path)
            .map_err(|e| StorageError::Other(format!("Failed to create ReDB database: {}", e)))?;
        
        // Create the tables if they don't exist
        let write_txn = db.begin_write()
            .map_err(|e| StorageError::Other(format!("Failed to begin ReDB transaction: {}", e)))?;
        
        // Create KV table
        write_txn.open_table(KV_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB KV table: {}", e)))?;
        
        // Create pages table
        write_txn.open_table(PAGE_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB pages table: {}", e)))?;
        
        // Commit the transaction
        write_txn.commit()
            .map_err(|e| StorageError::Other(format!("Failed to commit ReDB transaction: {}", e)))?;
        
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
}

impl StorageEngine for RedbStorageEngine {
    fn config(&self) -> &StorageConfig {
        &self.config
    }
    
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        // Begin a read transaction
        let read_txn = self.db.begin_read()
            .map_err(|e| StorageError::Other(format!("Failed to begin ReDB read transaction: {}", e)))?;
        
        // Open the KV table
        let table = read_txn.open_table(KV_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB KV table: {}", e)))?;
        
        // Get the value
        let result = table.get(key)
            .map_err(|e| StorageError::Other(format!("Failed to get value from ReDB: {}", e)))?
            .map(|v| v.value().to_vec());
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        Ok(result)
    }
    
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        // Begin a write transaction
        let mut write_txn = self.db.begin_write()
            .map_err(|e| StorageError::Other(format!("Failed to begin ReDB write transaction: {}", e)))?;
        
        // Open the KV table
        let mut table = write_txn.open_table(KV_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB KV table: {}", e)))?;
        
        // Check if the key exists
        let old_value = table.get(key)
            .map_err(|e| StorageError::Other(format!("Failed to get value from ReDB: {}", e)))?;
        let is_new = old_value.is_none();
        let old_size = old_value.map(|v| v.value().len()).unwrap_or(0);
        
        // Put the value
        table.insert(key, value)
            .map_err(|e| StorageError::Other(format!("Failed to put value to ReDB: {}", e)))?;
        
        // Commit the transaction
        write_txn.commit()
            .map_err(|e| StorageError::Other(format!("Failed to commit ReDB transaction: {}", e)))?;
        
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
        // Begin a write transaction
        let mut write_txn = self.db.begin_write()
            .map_err(|e| StorageError::Other(format!("Failed to begin ReDB write transaction: {}", e)))?;
        
        // Open the KV table
        let mut table = write_txn.open_table(KV_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB KV table: {}", e)))?;
        
        // Check if the key exists
        let old_value = table.get(key)
            .map_err(|e| StorageError::Other(format!("Failed to get value from ReDB: {}", e)))?;
        let old_size = old_value.map(|v| v.value().len()).unwrap_or(0);
        
        // Delete the key
        let removed = table.remove(key)
            .map_err(|e| StorageError::Other(format!("Failed to delete key from ReDB: {}", e)))?;
        
        // Commit the transaction
        write_txn.commit()
            .map_err(|e| StorageError::Other(format!("Failed to commit ReDB transaction: {}", e)))?;
        
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
        // Begin a read transaction
        let read_txn = self.db.begin_read()
            .map_err(|e| StorageError::Other(format!("Failed to begin ReDB read transaction: {}", e)))?;
        
        // Open the KV table
        let table = read_txn.open_table(KV_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB KV table: {}", e)))?;
        
        // Check if the key exists
        let exists = table.get(key)
            .map_err(|e| StorageError::Other(format!("Failed to get value from ReDB: {}", e)))?
            .is_some();
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        Ok(exists)
    }
    
    fn begin_transaction(&self) -> StorageResult<Box<dyn StorageTransaction + '_>> {
        // Create a new ReDB transaction
        let write_txn = self.db.begin_write()
            .map_err(|e| StorageError::Other(format!("Failed to begin ReDB write transaction: {}", e)))?;
        
        Ok(Box::new(RedbTransaction {
            txn: Some(write_txn),
            engine: self,
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
        
        // Begin a read transaction
        let read_txn = self.db.begin_read()
            .map_err(|e| StorageError::Other(format!("Failed to begin ReDB read transaction: {}", e)))?;
        
        // Open the pages table
        let table = read_txn.open_table(PAGE_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB pages table: {}", e)))?;
        
        // Get the page
        let page_data = table.get(page_id)
            .map_err(|e| StorageError::Other(format!("Failed to get page from ReDB: {}", e)))?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        // If the page exists, deserialize it and add to cache
        if let Some(page_data) = page_data {
            let page_bytes = page_data.value();
            let page: Page = bincode::deserialize(page_bytes)
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
        
        // Begin a write transaction
        let mut write_txn = self.db.begin_write()
            .map_err(|e| StorageError::Other(format!("Failed to begin ReDB write transaction: {}", e)))?;
        
        // Open the pages table
        let mut table = write_txn.open_table(PAGE_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB pages table: {}", e)))?;
        
        // Put the page
        table.insert(page_id, page_data.as_slice())
            .map_err(|e| StorageError::Other(format!("Failed to put page to ReDB: {}", e)))?;
        
        // Commit the transaction
        write_txn.commit()
            .map_err(|e| StorageError::Other(format!("Failed to commit ReDB transaction: {}", e)))?;
        
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
        // ReDB automatically flushes on transaction commit
        Ok(())
    }
    
    fn close(&self) -> StorageResult<()> {
        // ReDB automatically closes on drop
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
        "redb"
    }
}

/// ReDB transaction implementation
struct RedbTransaction<'a> {
    txn: Option<redb::WriteTransaction>,
    engine: &'a RedbStorageEngine,
}

impl<'a> StorageTransaction for RedbTransaction<'a> {
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        // If the transaction is already committed or aborted, return an error
        let txn = self.txn.as_ref().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        // Open the KV table
        let table = txn.open_table(KV_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB KV table: {}", e)))?;
        
        // Get the value
        let result = table.get(key)
            .map_err(|e| StorageError::Other(format!("Failed to get value from ReDB: {}", e)))?
            .map(|v| v.value().to_vec());
        
        Ok(result)
    }
    
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        // If the transaction is already committed or aborted, return an error
        let txn = self.txn.as_ref().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        // Open the KV table
        let mut table = txn.open_table(KV_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB KV table: {}", e)))?;
        
        // Put the value
        table.insert(key, value)
            .map_err(|e| StorageError::Other(format!("Failed to put value to ReDB: {}", e)))?;
        
        Ok(())
    }
    
    fn delete(&self, key: &[u8]) -> StorageResult<()> {
        // If the transaction is already committed or aborted, return an error
        let txn = self.txn.as_ref().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        // Open the KV table
        let mut table = txn.open_table(KV_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB KV table: {}", e)))?;
        
        // Delete the key
        table.remove(key)
            .map_err(|e| StorageError::Other(format!("Failed to delete key from ReDB: {}", e)))?;
        
        Ok(())
    }
    
    fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        // If the transaction is already committed or aborted, return an error
        let txn = self.txn.as_ref().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        // Open the KV table
        let table = txn.open_table(KV_TABLE)
            .map_err(|e| StorageError::Other(format!("Failed to open ReDB KV table: {}", e)))?;
        
        // Check if the key exists
        let exists = table.get(key)
            .map_err(|e| StorageError::Other(format!("Failed to get value from ReDB: {}", e)))?
            .is_some();
        
        Ok(exists)
    }
    
    fn commit(mut self: Box<Self>) -> StorageResult<()> {
        // If the transaction is already committed or aborted, return an error
        let txn = self.txn.take().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        // Commit the transaction
        txn.commit()
            .map_err(|e| StorageError::Other(format!("Failed to commit ReDB transaction: {}", e)))?;
        
        Ok(())
    }
    
    fn abort(mut self: Box<Self>) -> StorageResult<()> {
        // If the transaction is already committed or aborted, return an error
        let txn = self.txn.take().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        // Abort the transaction (just drop it)
        drop(txn);
        
        Ok(())
    }
}

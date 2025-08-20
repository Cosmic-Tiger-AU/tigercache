use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use parking_lot::RwLock;
use rusqlite::{Connection, params, Transaction, OpenFlags};

use crate::storage::error::{StorageError, StorageResult};
use crate::storage::config::StorageConfig;
use crate::storage::page::{Page, PageId, PageRef};
use crate::storage::storage_engine::{StorageEngine, StorageTransaction, StorageStats};

/// SQLite storage engine implementation
pub struct SqliteStorageEngine {
    /// Storage configuration
    config: StorageConfig,
    
    /// SQLite connection
    conn: Mutex<Connection>,
    
    /// Page cache
    page_cache: RwLock<HashMap<PageId, PageRef>>,
    
    /// Storage statistics
    stats: Arc<Mutex<StorageStats>>,
}

impl SqliteStorageEngine {
    /// Create a new SQLite storage engine
    pub fn new(config: StorageConfig) -> StorageResult<Self> {
        // Ensure we have a path
        let path = config.path.clone().ok_or_else(|| {
            StorageError::ConfigurationError("SQLite storage requires a path".to_string())
        })?;
        
        // Open the database
        let mut open_flags = OpenFlags::SQLITE_OPEN_READ_WRITE;
        if config.create_if_missing {
            open_flags |= OpenFlags::SQLITE_OPEN_CREATE;
        }
        
        let conn = Connection::open_with_flags(path, open_flags)
            .map_err(|e| StorageError::DatabaseError(format!("Failed to open SQLite database: {}", e)))?;
        
        // Enable WAL mode for better performance
        conn.execute_batch("PRAGMA journal_mode = WAL;")
            .map_err(|e| StorageError::DatabaseError(format!("Failed to set WAL mode: {}", e)))?;
        
        // Set synchronous mode based on config
        let sync_mode = if config.sync_writes { "FULL" } else { "NORMAL" };
        conn.execute_batch(&format!("PRAGMA synchronous = {};", sync_mode))
            .map_err(|e| StorageError::DatabaseError(format!("Failed to set synchronous mode: {}", e)))?;
        
        // Create tables if they don't exist
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kv_store (
                key BLOB PRIMARY KEY,
                value BLOB
            );
            CREATE TABLE IF NOT EXISTS pages (
                id INTEGER PRIMARY KEY,
                data BLOB
            );"
        ).map_err(|e| StorageError::DatabaseError(format!("Failed to create tables: {}", e)))?;
        
        Ok(Self {
            config,
            conn: Mutex::new(conn),
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
    
    /// Update storage statistics
    fn update_stats(&self) -> StorageResult<()> {
        let conn = self.conn.lock().map_err(|_| StorageError::DatabaseError("Failed to lock SQLite connection".to_string()))?;
        
        if let Ok(mut stats) = self.stats.lock() {
            // Count keys
            let key_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM kv_store",
                [],
                |row| row.get(0)
            ).map_err(|e| StorageError::DatabaseError(format!("Failed to count keys: {}", e)))?;
            stats.key_count = key_count as usize;
            
            // Count pages
            let page_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM pages",
                [],
                |row| row.get(0)
            ).map_err(|e| StorageError::DatabaseError(format!("Failed to count pages: {}", e)))?;
            stats.page_count = page_count as usize;
            
            // Calculate total value size
            let total_value_size: i64 = conn.query_row(
                "SELECT COALESCE(SUM(LENGTH(value)), 0) FROM kv_store",
                [],
                |row| row.get(0)
            ).map_err(|e| StorageError::DatabaseError(format!("Failed to calculate total value size: {}", e)))?;
            stats.total_value_size = total_value_size as usize;
            
            // Count dirty pages
            let page_cache = self.page_cache.read();
            stats.dirty_page_count = page_cache.values()
                .filter(|p| p.read().is_dirty())
                .count();
        }
        
        Ok(())
    }
}

impl StorageEngine for SqliteStorageEngine {
    fn config(&self) -> &StorageConfig {
        &self.config
    }
    
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let conn = self.conn.lock().map_err(|_| StorageError::DatabaseError("Failed to lock SQLite connection".to_string()))?;
        
        let result = conn.query_row(
            "SELECT value FROM kv_store WHERE key = ?",
            params![key],
            |row| row.get(0)
        ).optional().map_err(|e| StorageError::DatabaseError(format!("Failed to get value: {}", e)))?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        Ok(result)
    }
    
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        let conn = self.conn.lock().map_err(|_| StorageError::DatabaseError("Failed to lock SQLite connection".to_string()))?;
        
        conn.execute(
            "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?, ?)",
            params![key, value]
        ).map_err(|e| StorageError::DatabaseError(format!("Failed to put value: {}", e)))?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.write_count += 1;
        }
        
        Ok(())
    }
    
    fn delete(&self, key: &[u8]) -> StorageResult<()> {
        let conn = self.conn.lock().map_err(|_| StorageError::DatabaseError("Failed to lock SQLite connection".to_string()))?;
        
        conn.execute(
            "DELETE FROM kv_store WHERE key = ?",
            params![key]
        ).map_err(|e| StorageError::DatabaseError(format!("Failed to delete key: {}", e)))?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.write_count += 1;
        }
        
        Ok(())
    }
    
    fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let conn = self.conn.lock().map_err(|_| StorageError::DatabaseError("Failed to lock SQLite connection".to_string()))?;
        
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM kv_store WHERE key = ?",
            params![key],
            |row| row.get(0)
        ).map_err(|e| StorageError::DatabaseError(format!("Failed to check if key exists: {}", e)))?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        Ok(count > 0)
    }
    
    fn begin_transaction(&self) -> StorageResult<Box<dyn StorageTransaction + '_>> {
        let conn = self.conn.lock().map_err(|_| StorageError::DatabaseError("Failed to lock SQLite connection".to_string()))?;
        
        let tx = conn.transaction()
            .map_err(|e| StorageError::TransactionError(format!("Failed to begin transaction: {}", e)))?;
        
        Ok(Box::new(SqliteTransaction {
            tx: Some(tx),
            engine: self,
        }))
    }
    
    fn get_page(&self, page_id: PageId) -> StorageResult<Option<PageRef>> {
        // First check the cache
        let page_cache = self.page_cache.read();
        if let Some(page_ref) = page_cache.get(&page_id) {
            // Update stats
            if let Ok(mut stats) = self.stats.lock() {
                stats.read_count += 1;
            }
            
            return Ok(Some(page_ref.clone()));
        }
        drop(page_cache);
        
        // Not in cache, try to load from database
        let conn = self.conn.lock().map_err(|_| StorageError::DatabaseError("Failed to lock SQLite connection".to_string()))?;
        
        let page_data: Option<Vec<u8>> = conn.query_row(
            "SELECT data FROM pages WHERE id = ?",
            params![page_id],
            |row| row.get(0)
        ).optional().map_err(|e| StorageError::DatabaseError(format!("Failed to get page: {}", e)))?;
        
        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.read_count += 1;
        }
        
        // If the page exists, deserialize it and add to cache
        if let Some(page_data) = page_data {
            let page: Page = bincode::decode_from_slice(&page_data, bincode::config::standard())?
                .0;
            
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
        page_cache.insert(page_id, page_ref.clone());
        drop(page_cache);
        
        // If the page is dirty, write it to disk
        if page_ref.read().is_dirty() {
            let conn = self.conn.lock().map_err(|_| StorageError::DatabaseError("Failed to lock SQLite connection".to_string()))?;
            
            // Serialize the page
            let page_data = bincode::encode_to_vec(&*page_ref.read(), bincode::config::standard())?;
            
            conn.execute(
                "INSERT OR REPLACE INTO pages (id, data) VALUES (?, ?)",
                params![page_id, page_data]
            ).map_err(|e| StorageError::DatabaseError(format!("Failed to put page: {}", e)))?;
            
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
        let page_cache = self.page_cache.read();
        let conn = self.conn.lock().map_err(|_| StorageError::DatabaseError("Failed to lock SQLite connection".to_string()))?;
        
        // Begin a transaction for better performance
        let tx = conn.transaction()
            .map_err(|e| StorageError::TransactionError(format!("Failed to begin transaction: {}", e)))?;
        
        for (page_id, page_ref) in page_cache.iter() {
            if page_ref.read().is_dirty() {
                // Serialize the page
                let page_data = bincode::encode_to_vec(&*page_ref.read(), bincode::config::standard())?;
                
                tx.execute(
                    "INSERT OR REPLACE INTO pages (id, data) VALUES (?, ?)",
                    params![page_id, page_data]
                ).map_err(|e| StorageError::DatabaseError(format!("Failed to put page: {}", e)))?;
                
                // Mark the page as clean
                page_ref.write().mark_clean();
            }
        }
        
        // Commit the transaction
        tx.commit().map_err(|e| StorageError::TransactionError(format!("Failed to commit transaction: {}", e)))?;
        
        // Execute PRAGMA to ensure data is flushed to disk
        if self.config.sync_writes {
            conn.execute_batch("PRAGMA wal_checkpoint(FULL);")
                .map_err(|e| StorageError::DatabaseError(format!("Failed to checkpoint WAL: {}", e)))?;
        }
        
        // Update stats
        self.update_stats()?;
        
        Ok(())
    }
    
    fn close(&self) -> StorageResult<()> {
        // Flush all dirty pages
        self.flush()?;
        
        // Close the database (happens automatically when the connection is dropped)
        
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
        "sqlite"
    }
}

/// SQLite transaction implementation
struct SqliteTransaction<'a> {
    tx: Option<Transaction<'a>>,
    engine: &'a SqliteStorageEngine,
}

impl<'a> StorageTransaction for SqliteTransaction<'a> {
    fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>> {
        let tx = self.tx.as_ref().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        let result = tx.query_row(
            "SELECT value FROM kv_store WHERE key = ?",
            params![key],
            |row| row.get(0)
        ).optional().map_err(|e| StorageError::DatabaseError(format!("Failed to get value: {}", e)))?;
        
        Ok(result)
    }
    
    fn put(&self, key: &[u8], value: &[u8]) -> StorageResult<()> {
        let tx = self.tx.as_ref().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        tx.execute(
            "INSERT OR REPLACE INTO kv_store (key, value) VALUES (?, ?)",
            params![key, value]
        ).map_err(|e| StorageError::DatabaseError(format!("Failed to put value: {}", e)))?;
        
        Ok(())
    }
    
    fn delete(&self, key: &[u8]) -> StorageResult<()> {
        let tx = self.tx.as_ref().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        tx.execute(
            "DELETE FROM kv_store WHERE key = ?",
            params![key]
        ).map_err(|e| StorageError::DatabaseError(format!("Failed to delete key: {}", e)))?;
        
        Ok(())
    }
    
    fn exists(&self, key: &[u8]) -> StorageResult<bool> {
        let tx = self.tx.as_ref().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM kv_store WHERE key = ?",
            params![key],
            |row| row.get(0)
        ).map_err(|e| StorageError::DatabaseError(format!("Failed to check if key exists: {}", e)))?;
        
        Ok(count > 0)
    }
    
    fn commit(mut self: Box<Self>) -> StorageResult<()> {
        let tx = self.tx.take().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        tx.commit().map_err(|e| StorageError::TransactionError(format!("Failed to commit transaction: {}", e)))?;
        
        // Update stats
        if let Ok(mut stats) = self.engine.stats.lock() {
            stats.write_count += 1;
        }
        
        Ok(())
    }
    
    fn abort(mut self: Box<Self>) -> StorageResult<()> {
        let tx = self.tx.take().ok_or_else(|| {
            StorageError::TransactionError("Transaction already committed or aborted".to_string())
        })?;
        
        // SQLite automatically rolls back when the transaction is dropped
        drop(tx);
        
        Ok(())
    }
}

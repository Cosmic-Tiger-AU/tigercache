use std::collections::HashMap;
use std::hash::Hash;
use std::time::Instant;
use bytesize::ByteSize;
use parking_lot::RwLock;

/// LRU cache entry
struct LruEntry<K, V> {
    /// Key
    key: K,
    
    /// Value
    value: V,
    
    /// Size in bytes
    size: usize,
    
    /// Last access time
    last_access: Instant,
    
    /// Reference count
    ref_count: u32,
}

/// LRU cache with size-based eviction
pub struct LruCache<K, V> {
    /// Cache entries
    entries: RwLock<HashMap<K, LruEntry<K, V>>>,
    
    /// Maximum size in bytes
    max_size: usize,
    
    /// Current size in bytes
    current_size: RwLock<usize>,
    
    /// Cache hit count
    hits: RwLock<u64>,
    
    /// Cache miss count
    misses: RwLock<u64>,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + Hash + Clone,
{
    /// Create a new LRU cache with the specified maximum size
    pub fn new(max_size: ByteSize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_size: max_size.as_u64() as usize,
            current_size: RwLock::new(0),
            hits: RwLock::new(0),
            misses: RwLock::new(0),
        }
    }
    
    /// Get a value from the cache
    pub fn get<Q>(&self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let mut entries = self.entries.write();
        
        // Find the entry
        let entry = entries.iter_mut().find(|(k, _)| (*k).borrow() == key);
        
        match entry {
            Some((_, entry)) => {
                // Update access time
                entry.last_access = Instant::now();
                
                // Increment reference count
                entry.ref_count = entry.ref_count.saturating_add(1);
                
                // Increment hit count
                *self.hits.write() += 1;
                
                // Clone the value
                Some(entry.value.clone())
            }
            None => {
                // Increment miss count
                *self.misses.write() += 1;
                
                None
            }
        }
    }
    
    /// Put a value in the cache
    pub fn put(&self, key: K, value: V, size: usize) -> Option<V>
    where
        V: Clone,
    {
        let mut entries = self.entries.write();
        let mut current_size = self.current_size.write();
        
        // Check if the key already exists
        if let Some(entry) = entries.get_mut(&key) {
            // Update the entry
            let old_value = entry.value.clone();
            let old_size = entry.size;
            
            entry.value = value;
            entry.size = size;
            entry.last_access = Instant::now();
            
            // Update current size
            *current_size = current_size.saturating_sub(old_size).saturating_add(size);
            
            return Some(old_value);
        }
        
        // Check if we need to evict entries
        if *current_size + size > self.max_size {
            self.evict_entries(&mut entries, &mut current_size, size);
        }
        
        // Add the new entry
        entries.insert(key.clone(), LruEntry {
            key,
            value: value.clone(),
            size,
            last_access: Instant::now(),
            ref_count: 0,
        });
        
        // Update current size
        *current_size = current_size.saturating_add(size);
        
        None
    }
    
    /// Remove a value from the cache
    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        V: Clone,
    {
        let mut entries = self.entries.write();
        let mut current_size = self.current_size.write();
        
        // Find the entry
        if let Some(entry) = entries.iter().find(|(k, _)| (*k).borrow() == key) {
            let key = entry.0.clone();
            let entry = entry.1;
            let value = entry.value.clone();
            let size = entry.size;
            
            // Remove the entry
            entries.remove(&key);
            
            // Update current size
            *current_size = current_size.saturating_sub(size);
            
            Some(value)
        } else {
            None
        }
    }
    
    /// Clear the cache
    pub fn clear(&self) {
        let mut entries = self.entries.write();
        let mut current_size = self.current_size.write();
        
        entries.clear();
        *current_size = 0;
    }
    
    /// Get the current size of the cache in bytes
    pub fn size(&self) -> ByteSize {
        ByteSize::b(*self.current_size.read() as u64)
    }
    
    /// Get the maximum size of the cache in bytes
    pub fn max_size(&self) -> ByteSize {
        ByteSize::b(self.max_size as u64)
    }
    
    /// Get the number of entries in the cache
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }
    
    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
    
    /// Get the cache hit rate (0.0 - 1.0)
    pub fn hit_rate(&self) -> f64 {
        let hits = *self.hits.read();
        let misses = *self.misses.read();
        let total = hits + misses;
        
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }
    
    /// Evict entries to make room for a new entry
    fn evict_entries(
        &self,
        entries: &mut HashMap<K, LruEntry<K, V>>,
        current_size: &mut usize,
        needed_size: usize,
    ) {
        // Calculate how much space we need to free
        let target_size = self.max_size.saturating_sub(needed_size);
        let mut size_to_free = current_size.saturating_sub(target_size);
        
        if size_to_free == 0 {
            return;
        }
        
        // Sort entries by last access time (oldest first)
        let mut sorted_entries: Vec<_> = entries.iter().collect();
        sorted_entries.sort_by(|a, b| a.1.last_access.cmp(&b.1.last_access));
        
        // Evict entries until we have enough space
        let mut freed_size = 0;
        let mut keys_to_remove = Vec::new();
        
        for (key, entry) in sorted_entries {
            // Skip entries with non-zero reference count
            if entry.ref_count > 0 {
                continue;
            }
            
            freed_size += entry.size;
            keys_to_remove.push(key.clone());
            
            if freed_size >= size_to_free {
                break;
            }
        }
        
        // Remove the evicted entries
        for key in keys_to_remove {
            if let Some(entry) = entries.remove(&key) {
                *current_size = current_size.saturating_sub(entry.size);
            }
        }
    }
}


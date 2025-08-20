use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

/// A unique identifier for an interned string
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StringId(pub u32);

impl StringId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
    
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// String interning system for memory efficiency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringInterner {
    /// Maps strings to their IDs
    string_to_id: FxHashMap<String, StringId>,
    /// Maps IDs back to strings
    id_to_string: FxHashMap<StringId, String>,
    /// Next available ID
    next_id: u32,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            string_to_id: FxHashMap::default(),
            id_to_string: FxHashMap::default(),
            next_id: 0,
        }
    }
    
    /// Intern a string and return its ID, reusing existing IDs for duplicate strings
    pub fn intern(&mut self, s: &str) -> StringId {
        if let Some(&id) = self.string_to_id.get(s) {
            return id;
        }
        
        let id = StringId::new(self.next_id);
        self.next_id += 1;
        
        let owned_string = s.to_string();
        self.string_to_id.insert(owned_string.clone(), id);
        self.id_to_string.insert(id, owned_string);
        
        id
    }
    
    /// Get the string for a given ID
    pub fn get(&self, id: StringId) -> Option<&str> {
        self.id_to_string.get(&id).map(|s| s.as_str())
    }
    
    /// Get the ID for a given string
    pub fn get_id(&self, s: &str) -> Option<StringId> {
        self.string_to_id.get(s).copied()
    }
    
    /// Get the number of interned strings
    pub fn len(&self) -> usize {
        self.string_to_id.len()
    }
    
    /// Check if the interner is empty
    pub fn is_empty(&self) -> bool {
        self.string_to_id.is_empty()
    }
    
    /// Clear all interned strings
    pub fn clear(&mut self) {
        self.string_to_id.clear();
        self.id_to_string.clear();
        self.next_id = 0;
    }
    
    /// Iterate over all interned strings and their IDs
    pub fn iter(&self) -> impl Iterator<Item = (StringId, &str)> {
        self.id_to_string.iter().map(|(&id, s)| (id, s.as_str()))
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_interning() {
        let mut interner = StringInterner::new();
        
        let id1 = interner.intern("hello");
        let id2 = interner.intern("world");
        let id3 = interner.intern("hello"); // Duplicate
        
        assert_eq!(id1, id3); // Same string should get same ID
        assert_ne!(id1, id2); // Different strings should get different IDs
        
        assert_eq!(interner.get(id1), Some("hello"));
        assert_eq!(interner.get(id2), Some("world"));
        assert_eq!(interner.len(), 2); // Only 2 unique strings
    }
    
    #[test]
    fn test_get_id() {
        let mut interner = StringInterner::new();
        
        let id = interner.intern("test");
        assert_eq!(interner.get_id("test"), Some(id));
        assert_eq!(interner.get_id("nonexistent"), None);
    }
    
    #[test]
    fn test_clear() {
        let mut interner = StringInterner::new();
        
        interner.intern("hello");
        interner.intern("world");
        assert_eq!(interner.len(), 2);
        
        interner.clear();
        assert_eq!(interner.len(), 0);
        assert!(interner.is_empty());
    }
}
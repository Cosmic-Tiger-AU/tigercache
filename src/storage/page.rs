use std::sync::Arc;
use serde::{Deserialize, Serialize};
use parking_lot::RwLock;

/// Page ID type
pub type PageId = u64;

/// Page status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageStatus {
    /// Page is clean (not modified)
    Clean,
    /// Page is dirty (modified but not yet written to disk)
    Dirty,
    /// Page is being written to disk
    Writing,
    /// Page is being read from disk
    Reading,
}

/// Page structure representing a fixed-size chunk of data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Unique identifier for the page
    pub id: PageId,
    
    /// The actual data stored in the page
    pub data: Vec<u8>,
    
    /// Status of the page
    #[serde(skip)]
    pub status: PageStatus,
    
    /// Reference count for cache management
    #[serde(skip)]
    pub ref_count: u32,
    
    /// Last access time for LRU cache management
    #[serde(skip)]
    pub last_access: std::time::Instant,
}

impl Page {
    /// Create a new page with the given ID and data
    pub fn new(id: PageId, data: Vec<u8>) -> Self {
        Self {
            id,
            data,
            status: PageStatus::Clean,
            ref_count: 0,
            last_access: std::time::Instant::now(),
        }
    }
    
    /// Create a new empty page with the given ID and size
    pub fn new_empty(id: PageId, size: usize) -> Self {
        Self::new(id, vec![0; size])
    }
    
    /// Mark the page as dirty
    pub fn mark_dirty(&mut self) {
        self.status = PageStatus::Dirty;
    }
    
    /// Mark the page as clean
    pub fn mark_clean(&mut self) {
        self.status = PageStatus::Clean;
    }
    
    /// Update the last access time
    pub fn touch(&mut self) {
        self.last_access = std::time::Instant::now();
    }
    
    /// Increment the reference count
    pub fn increment_ref_count(&mut self) {
        self.ref_count = self.ref_count.saturating_add(1);
    }
    
    /// Decrement the reference count
    pub fn decrement_ref_count(&mut self) {
        self.ref_count = self.ref_count.saturating_sub(1);
    }
    
    /// Check if the page is dirty
    pub fn is_dirty(&self) -> bool {
        self.status == PageStatus::Dirty
    }
    
    /// Get the size of the page in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Thread-safe reference to a page
pub type PageRef = Arc<RwLock<Page>>;

/// Page cache entry
#[derive(Debug)]
pub struct PageCacheEntry {
    /// Reference to the page
    pub page: PageRef,
    
    /// Whether the page is pinned in memory
    pub pinned: bool,
}

impl PageCacheEntry {
    /// Create a new page cache entry
    pub fn new(page: Page, pinned: bool) -> Self {
        Self {
            page: Arc::new(RwLock::new(page)),
            pinned,
        }
    }
    
    /// Pin the page in memory
    pub fn pin(&mut self) {
        self.pinned = true;
    }
    
    /// Unpin the page
    pub fn unpin(&mut self) {
        self.pinned = false;
    }
    
    /// Check if the page is pinned
    pub fn is_pinned(&self) -> bool {
        self.pinned
    }
    
    /// Get the page ID
    pub fn id(&self) -> PageId {
        self.page.read().id
    }
    
    /// Check if the page is dirty
    pub fn is_dirty(&self) -> bool {
        self.page.read().is_dirty()
    }
    
    /// Get the reference count
    pub fn ref_count(&self) -> u32 {
        self.page.read().ref_count
    }
    
    /// Get the last access time
    pub fn last_access(&self) -> std::time::Instant {
        self.page.read().last_access
    }
}


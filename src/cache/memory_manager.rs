use std::sync::{Arc, Mutex};
use std::time::Instant;
use bytesize::ByteSize;
use parking_lot::RwLock;
use crossbeam_channel::{Sender, Receiver, unbounded};

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    /// Low memory pressure (< 50% of max memory)
    Low,
    /// Medium memory pressure (50-80% of max memory)
    Medium,
    /// High memory pressure (80-95% of max memory)
    High,
    /// Critical memory pressure (> 95% of max memory)
    Critical,
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Current memory usage in bytes
    pub current_usage: ByteSize,
    
    /// Maximum memory usage in bytes
    pub max_usage: ByteSize,
    
    /// Memory pressure level
    pub pressure_level: MemoryPressureLevel,
    
    /// Document cache size in bytes
    pub document_cache_size: ByteSize,
    
    /// Index cache size in bytes
    pub index_cache_size: ByteSize,
    
    /// Query cache size in bytes
    pub query_cache_size: ByteSize,
    
    /// Document cache hit rate (0.0 - 1.0)
    pub document_cache_hit_rate: f64,
    
    /// Index cache hit rate (0.0 - 1.0)
    pub index_cache_hit_rate: f64,
    
    /// Query cache hit rate (0.0 - 1.0)
    pub query_cache_hit_rate: f64,
    
    /// Last memory pressure check time
    pub last_check_time: Instant,
}

/// Memory manager event
#[derive(Debug, Clone)]
pub enum MemoryEvent {
    /// Memory pressure level changed
    PressureChanged(MemoryPressureLevel),
    
    /// Memory usage exceeded threshold
    MemoryExceeded {
        current: ByteSize,
        max: ByteSize,
    },
    
    /// Cache eviction required
    EvictionRequired {
        bytes_to_free: ByteSize,
    },
    
    /// Cache statistics updated
    StatsUpdated(MemoryStats),
}

/// Memory manager for TigerCache
///
/// Responsible for monitoring memory usage and triggering cache evictions
/// when memory pressure is high.
pub struct MemoryManager {
    /// Maximum memory usage in bytes
    max_memory: ByteSize,
    
    /// Current memory usage in bytes
    current_usage: Arc<RwLock<ByteSize>>,
    
    /// Memory statistics
    stats: Arc<RwLock<MemoryStats>>,
    
    /// Event sender
    event_sender: Sender<MemoryEvent>,
    
    /// Event receiver
    event_receiver: Receiver<MemoryEvent>,
    
    /// Whether the memory manager is running
    running: Arc<RwLock<bool>>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(max_memory: ByteSize) -> Self {
        let (sender, receiver) = unbounded();
        
        let stats = MemoryStats {
            current_usage: ByteSize::b(0),
            max_usage: max_memory,
            pressure_level: MemoryPressureLevel::Low,
            document_cache_size: ByteSize::b(0),
            index_cache_size: ByteSize::b(0),
            query_cache_size: ByteSize::b(0),
            document_cache_hit_rate: 0.0,
            index_cache_hit_rate: 0.0,
            query_cache_hit_rate: 0.0,
            last_check_time: Instant::now(),
        };
        
        Self {
            max_memory,
            current_usage: Arc::new(RwLock::new(ByteSize::b(0))),
            stats: Arc::new(RwLock::new(stats)),
            event_sender: sender,
            event_receiver: receiver,
            running: Arc::new(RwLock::new(true)),
        }
    }
    
    /// Start the memory manager
    pub fn start(&self) {
        let running = self.running.clone();
        let current_usage = self.current_usage.clone();
        let max_memory = self.max_memory;
        let stats = self.stats.clone();
        let sender = self.event_sender.clone();
        
        // Spawn a background thread to monitor memory usage
        std::thread::spawn(move || {
            while *running.read() {
                // Check memory pressure every 100ms
                std::thread::sleep(std::time::Duration::from_millis(100));
                
                // Get current memory usage
                let usage = *current_usage.read();
                
                // Calculate memory pressure level
                let pressure_level = if usage.as_u64() > max_memory.as_u64() * 95 / 100 {
                    MemoryPressureLevel::Critical
                } else if usage.as_u64() > max_memory.as_u64() * 80 / 100 {
                    MemoryPressureLevel::High
                } else if usage.as_u64() > max_memory.as_u64() * 50 / 100 {
                    MemoryPressureLevel::Medium
                } else {
                    MemoryPressureLevel::Low
                };
                
                // Update stats
                let mut stats_guard = stats.write();
                let old_pressure = stats_guard.pressure_level;
                stats_guard.current_usage = usage;
                stats_guard.pressure_level = pressure_level;
                stats_guard.last_check_time = Instant::now();
                drop(stats_guard);
                
                // Send events if needed
                if pressure_level != old_pressure {
                    let _ = sender.send(MemoryEvent::PressureChanged(pressure_level));
                }
                
                if usage > max_memory {
                    let _ = sender.send(MemoryEvent::MemoryExceeded {
                        current: usage,
                        max: max_memory,
                    });
                    
                    // Calculate how much memory to free
                    let bytes_to_free = ByteSize::b(usage.as_u64().saturating_sub(max_memory.as_u64() * 80 / 100));
                    let _ = sender.send(MemoryEvent::EvictionRequired {
                        bytes_to_free,
                    });
                }
            }
        });
    }
    
    /// Stop the memory manager
    pub fn stop(&self) {
        *self.running.write() = false;
    }
    
    /// Allocate memory
    pub fn allocate(&self, size: ByteSize) -> bool {
        let mut current = self.current_usage.write();
        *current = ByteSize::b(current.as_u64() + size.as_u64());
        
        // Check if we exceeded the maximum
        if *current > self.max_memory {
            // We'll allow the allocation but trigger eviction
            let _ = self.event_sender.send(MemoryEvent::MemoryExceeded {
                current: *current,
                max: self.max_memory,
            });
            
            // Calculate how much memory to free
            let bytes_to_free = ByteSize::b(current.as_u64().saturating_sub(self.max_memory.as_u64() * 80 / 100));
            let _ = self.event_sender.send(MemoryEvent::EvictionRequired {
                bytes_to_free,
            });
            
            false
        } else {
            true
        }
    }
    
    /// Free memory
    pub fn free(&self, size: ByteSize) {
        let mut current = self.current_usage.write();
        *current = ByteSize::b(current.as_u64().saturating_sub(size.as_u64()));
    }
    
    /// Get current memory usage
    pub fn current_usage(&self) -> ByteSize {
        *self.current_usage.read()
    }
    
    /// Get maximum memory usage
    pub fn max_memory(&self) -> ByteSize {
        self.max_memory
    }
    
    /// Get memory pressure level
    pub fn pressure_level(&self) -> MemoryPressureLevel {
        self.stats.read().pressure_level
    }
    
    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        self.stats.read().clone()
    }
    
    /// Update memory statistics
    pub fn update_stats(&self, stats: MemoryStats) {
        *self.stats.write() = stats.clone();
        let _ = self.event_sender.send(MemoryEvent::StatsUpdated(stats));
    }
    
    /// Get event receiver
    pub fn event_receiver(&self) -> Receiver<MemoryEvent> {
        self.event_receiver.clone()
    }
}

use crate::chunk::Chunk;
use crate::allocation::Allocation;

/// A chunked bump allocator with selective reset.
/// 
/// Memory is divided into fixed-size chunks. Allocations bump a pointer
/// within chunks. Each chunk tracks live allocations and can be reset
/// independently when empty.
pub struct ChunkedBumpAllocator {
    chunks: Vec<Chunk>,
    chunk_size: usize,
    total_allocated: usize,
    total_reset: usize,
}

impl ChunkedBumpAllocator {
    /// Creates a new allocator with the specified chunk size.
    /// 
    /// Chunk size should be a power of two for best performance.
    pub fn new(chunk_size: usize) -> Self {
        assert!(chunk_size >= 64, "Chunk size too small");
        assert!(chunk_size.is_power_of_two(), "Chunk size should be power of two");
        
        Self {
            chunks: Vec::new(),
            chunk_size,
            total_allocated: 0,
            total_reset: 0,
        }
    }
    
    /// Allocates memory with default alignment (8 bytes).
    pub fn allocate(&mut self, size: usize) -> Allocation {
        self.allocate_aligned(size, 8)
    }
    
    /*
    /// Allocates memory with specified alignment.
    pub fn allocate_aligned(&mut self, size: usize, align: usize) -> Allocation {
        assert!(size > 0, "Cannot allocate 0 bytes");
        assert!(size <= self.chunk_size, "Allocation too large for chunk size");
        
        // Try to allocate from the last chunk
        if let Some(chunk) = self.chunks.last_mut() {
            if let Some(ptr) = chunk.allocate_aligned(size, align) {
                self.total_allocated += size;
                return unsafe {
                    Allocation::new(ptr, size, chunk as *const Chunk)
                };
            }
        }
        
        // Need a new chunk
        self.add_chunk();
        
        // Try again (should succeed)
        let chunk = self.chunks.last_mut().unwrap();
        let ptr = chunk.allocate_aligned(size, align)
            .expect("Fresh chunk should have space");
        
        self.total_allocated += size;
        
        unsafe {
            Allocation::new(ptr, size, chunk as *const Chunk)
        }
    }
    */

    pub fn allocate_aligned(&mut self, size: usize, align: usize) -> Allocation {
        assert!(size > 0, "Cannot allocate 0 bytes");
        assert!(size <= self.chunk_size, "Allocation too large for chunk size");
        
        // Try to allocate from existing chunks (including reset ones)
        for chunk in &mut self.chunks {
            if let Some(ptr) = chunk.allocate_aligned(size, align) {
                self.total_allocated += size;
                return unsafe {
                    Allocation::new(ptr, size, chunk as *const Chunk)
                };
            }
        }
        
        // Need a new chunk
        self.add_chunk();
        
        // Try again from new chunk
        let chunk = self.chunks.last_mut().unwrap();
        let ptr = chunk.allocate_aligned(size, align)
            .expect("Fresh chunk should have space");
        
        self.total_allocated += size;
        
        unsafe {
            Allocation::new(ptr, size, chunk as *const Chunk)
        }
    }
    
    /// Adds a new chunk to the allocator.
    fn add_chunk(&mut self) {
        self.chunks.push(Chunk::new(self.chunk_size));
    }
    
    /// Removes empty chunks from memory, optionally keeping some for reuse.
    /// 
    /// This actually deallocates memory by removing chunks from the Vec,
    /// allowing them to be dropped and freed.
    /// 
    /// # Arguments
    /// * `keep_spare` - Number of empty chunks to keep for potential reuse (default: 2)
    /// 
    /// # Returns
    /// Number of chunks removed and deallocated
    pub fn reset_empty_chunks(&mut self) -> usize {
        self.reset_empty_chunks_keep_spare(2)
    }

    /// Removes empty chunks, keeping specified number for reuse.
    pub fn reset_empty_chunks_keep_spare(&mut self, keep_count: usize) -> usize {
        let initial_count = self.chunks.len();
        
        // Count how many empty chunks we've seen
        let mut empty_reset_count = 0;
        
        // Retain chunks based on their status
        self.chunks.retain_mut(|chunk| {
            if chunk.ref_count() > 0 {
                // Has live allocations - must keep
                true
            } else if chunk.used() == 0 {
                // Never used - keep it
                true
            } else {
                // Empty and was previously used
                empty_reset_count += 1;
                
                if empty_reset_count <= keep_count {
                    // Keep this one for reuse, but reset it
                    chunk.reset();
                    true
                } else {
                    // Remove it (will be dropped and deallocated)
                    false
                }
            }
        });
        
        let removed = initial_count - self.chunks.len();
        self.total_reset += removed;
        
        removed
    }

    /// Aggressively removes ALL empty chunks (maximum memory reclamation).
    /// 
    /// Use when memory is critical and you want to minimize footprint.
    pub fn reset_and_free_all_empty(&mut self) -> usize {
        self.reset_empty_chunks_keep_spare(0)
    }
    
    /// Resets all chunks with utilization below the threshold.
    /// 
    /// Only resets chunks with ref_count == 0.
    /// Returns the number of chunks reset.
    pub fn reset_underutilized_chunks(&mut self, threshold: f32) -> usize {
        assert!((0.0..=1.0).contains(&threshold), "Threshold must be between 0 and 1");
        
        let mut count = 0;
        
        for chunk in &mut self.chunks {
            if chunk.is_empty() && chunk.utilization() < threshold && chunk.used() > 0 {
                chunk.reset();
                count += 1;
                self.total_reset += 1;
            }
        }
        
        count
    }
    
    /// Returns the number of chunks currently allocated.
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
    
    /// Returns the total memory capacity (allocated chunks).
    pub fn total_capacity(&self) -> usize {
        self.chunks.len() * self.chunk_size
    }
    
    /// Returns the total memory currently in use.
    pub fn total_used(&self) -> usize {
        self.chunks.iter().map(|c| c.used()).sum()
    }
    
    /// Returns the total number of live allocations across all chunks.
    pub fn total_live_allocations(&self) -> usize {
        self.chunks.iter().map(|c| c.ref_count()).sum()
    }
    
    /// Returns the overall utilization (0.0 to 1.0).
    pub fn overall_utilization(&self) -> f32 {
        let capacity = self.total_capacity();
        if capacity == 0 {
            return 0.0;
        }
        self.total_used() as f32 / capacity as f32
    }
    
    /// Returns statistics about the allocator.
    pub fn stats(&self) -> AllocatorStats {
        let mut empty_chunks = 0;
        let mut utilization_sum = 0.0;
        
        for chunk in &self.chunks {
            if chunk.is_empty() {
                empty_chunks += 1;
            }
            utilization_sum += chunk.utilization();
        }
        
        let avg_utilization = if self.chunks.is_empty() {
            0.0
        } else {
            utilization_sum / self.chunks.len() as f32
        };
        
        AllocatorStats {
            chunk_count: self.chunks.len(),
            chunk_size: self.chunk_size,
            empty_chunks,
            total_capacity: self.total_capacity(),
            total_used: self.total_used(),
            total_live_allocations: self.total_live_allocations(),
            overall_utilization: self.overall_utilization(),
            average_chunk_utilization: avg_utilization,
            total_allocated: self.total_allocated,
            total_reset: self.total_reset,
        }
    }
    
    /// Resets all chunks (ignores reference counts).
    /// 
    /// # Safety
    /// 
    /// This is unsafe because it invalidates all outstanding allocations.
    /// Only use this when you know all allocations have been dropped.
    pub unsafe fn reset_all(&mut self) {
        for chunk in &mut self.chunks {
            chunk.reset();
            self.total_reset += 1;
        }
    }
}

/// Statistics about the allocator state.
#[derive(Debug, Clone)]
pub struct AllocatorStats {
    pub chunk_count: usize,
    pub chunk_size: usize,
    pub empty_chunks: usize,
    pub total_capacity: usize,
    pub total_used: usize,
    pub total_live_allocations: usize,
    pub overall_utilization: f32,
    pub average_chunk_utilization: f32,
    pub total_allocated: usize,
    pub total_reset: usize,
}

impl std::fmt::Display for AllocatorStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Chunks: {}, Empty: {}, Capacity: {} KB, Used: {} KB, Utilization: {:.1}%, Live: {}",
            self.chunk_count,
            self.empty_chunks,
            self.total_capacity / 1024,
            self.total_used / 1024,
            self.overall_utilization * 100.0,
            self.total_live_allocations
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocator_creation() {
        let allocator = ChunkedBumpAllocator::new(4096);
        assert_eq!(allocator.chunk_count(), 0);
    }

    #[test]
    fn test_first_allocation_creates_chunk() {
        let mut allocator = ChunkedBumpAllocator::new(4096);
        let _alloc = allocator.allocate(64);
        assert_eq!(allocator.chunk_count(), 1);
    }

    #[test]
    fn test_multiple_allocations_same_chunk() {
        let mut allocator = ChunkedBumpAllocator::new(4096);
        
        let _alloc1 = allocator.allocate(64);
        let _alloc2 = allocator.allocate(128);
        let _alloc3 = allocator.allocate(256);
        
        assert_eq!(allocator.chunk_count(), 1);
        assert_eq!(allocator.total_live_allocations(), 3);
    }

    #[test]
    fn test_allocation_spans_chunks() {
        let mut allocator = ChunkedBumpAllocator::new(1024);
        
        let _alloc1 = allocator.allocate(800);
        let _alloc2 = allocator.allocate(800); // Should create new chunk
        
        assert_eq!(allocator.chunk_count(), 2);
    }

    #[test]
    fn test_selective_reset() {
        let mut allocator = ChunkedBumpAllocator::new(4096);
        
        let alloc1 = allocator.allocate(64);
        let alloc2 = allocator.allocate(128);
        
        // Both in same chunk
        assert_eq!(allocator.chunk_count(), 1);
        
        // Drop first allocation
        drop(alloc1);
        
        // Should not reset (alloc2 still alive)
        let reset = allocator.reset_empty_chunks();
        assert_eq!(reset, 0);
        
        // Drop second allocation
        drop(alloc2);
        
        // Now should reset
        let reset = allocator.reset_empty_chunks();
        assert_eq!(reset, 1);
    }

    #[test]
    fn test_stats() {
        let mut allocator = ChunkedBumpAllocator::new(4096);
        
        let _alloc1 = allocator.allocate(1000);
        let _alloc2 = allocator.allocate(2000);
        
        let stats = allocator.stats();
        
        assert_eq!(stats.chunk_count, 1);
        assert_eq!(stats.total_live_allocations, 2);
        assert!(stats.overall_utilization > 0.7); // ~3000/4096
    }
}
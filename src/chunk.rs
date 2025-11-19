use std::sync::atomic::{AtomicUsize, Ordering};
use std::alloc::{alloc, dealloc, Layout};

/// A single chunk of memory with bump allocation.
/// 
/// Tracks the number of live allocations via reference counting
/// to enable selective reset.
pub struct Chunk {
    /// Pointer to the start of the memory region
    memory: *mut u8,
    
    /// Total size of this chunk
    size: usize,
    
    /// Current offset for bump allocation (how much is used)
    used: usize,
    
    /// Number of live allocations in this chunk
    ref_count: AtomicUsize,
    
    /// Total number of allocations ever made from this chunk
    allocation_count: usize,
}

impl Chunk {
    /// Creates a new chunk with the specified size.
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "Chunk size must be greater than 0");
        assert!(size.is_power_of_two(), "Chunk size should be power of two for alignment");
        
        let layout = Layout::from_size_align(size, 8)
            .expect("Invalid layout");
        
        let memory = unsafe { alloc(layout) };
        
        if memory.is_null() {
            panic!("Failed to allocate chunk of size {}", size);
        }
        
        Chunk {
            memory,
            size,
            used: 0,
            ref_count: AtomicUsize::new(0),
            allocation_count: 0,
        }
    }
    
    /// Attempts to allocate memory of the given size with default alignment (8 bytes).
    pub fn allocate(&mut self, size: usize) -> Option<*mut u8> {
        self.allocate_aligned(size, 8)
    }
    
    /// Attempts to allocate memory with specific alignment.
    pub fn allocate_aligned(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        assert!(align.is_power_of_two(), "Alignment must be power of two");
        assert!(size > 0, "Allocation size must be greater than 0");
        
        // Calculate aligned offset
        let padding = (align - (self.used % align)) % align;
        let aligned_offset = self.used + padding;
        
        // Check if we have space
        if aligned_offset + size > self.size {
            return None; // Chunk is full
        }
        
        // Update state
        self.used = aligned_offset + size;
        self.ref_count.fetch_add(1, Ordering::Relaxed);
        self.allocation_count += 1;
        
        // Return pointer
        Some(unsafe { self.memory.add(aligned_offset) })
    }
    
    /// Decrements the reference count when an allocation is freed.
    pub fn decrement_ref_count(&self) {
        self.ref_count.fetch_sub(1, Ordering::Relaxed);
    }
    
    /// Returns the current reference count.
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::Relaxed)
    }
    
    /// Returns true if this chunk has no live allocations.
    pub fn is_empty(&self) -> bool {
        self.ref_count.load(Ordering::Relaxed) == 0
    }
    
    /// Resets the chunk for reuse.
    /// 
    /// # Safety
    /// 
    /// Must only be called when ref_count is 0 (no live allocations).
    pub fn reset(&mut self) {
        assert_eq!(self.ref_count(), 0, "Cannot reset chunk with live allocations");
        self.used = 0;
    }
    
    /// Returns the utilization of this chunk (0.0 to 1.0).
    pub fn utilization(&self) -> f32 {
        self.used as f32 / self.size as f32
    }
    
    /// Returns the amount of free space remaining.
    pub fn free_space(&self) -> usize {
        self.size - self.used
    }
    
    /// Returns the total size of this chunk.
    pub fn size(&self) -> usize {
        self.size
    }
    
    /// Returns the amount of space currently used.
    pub fn used(&self) -> usize {
        self.used
    }
    
    /// Returns the total number of allocations made from this chunk.
    pub fn allocation_count(&self) -> usize {
        self.allocation_count
    }
    
    /// Returns whether the pointer came from this chunk.
    pub fn contains(&self, ptr: *const u8) -> bool {
        let start = self.memory as usize;
        let end = start + self.size;
        let addr = ptr as usize;
        addr >= start && addr < end
    }
}

impl Drop for Chunk {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.size, 8)
            .expect("Invalid layout");
        unsafe {
            dealloc(self.memory, layout);
        }
    }
}

// Chunks can be sent between threads
unsafe impl Send for Chunk {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let chunk = Chunk::new(4096);
        assert_eq!(chunk.size(), 4096);
        assert_eq!(chunk.used(), 0);
        assert_eq!(chunk.ref_count(), 0);
    }

    #[test]
    fn test_chunk_allocation() {
        let mut chunk = Chunk::new(4096);
        
        let ptr1 = chunk.allocate(64).unwrap();
        assert!(!ptr1.is_null());
        assert_eq!(chunk.used(), 64);
        assert_eq!(chunk.ref_count(), 1);
        
        let ptr2 = chunk.allocate(128).unwrap();
        assert!(!ptr2.is_null());
        assert_eq!(chunk.used(), 192);
        assert_eq!(chunk.ref_count(), 2);
    }

    #[test]
    fn test_chunk_full() {
        let mut chunk = Chunk::new(256);
        
        let _ptr1 = chunk.allocate(200).unwrap();
        let result = chunk.allocate(100); // Should fail
        
        assert!(result.is_none());
    }

    #[test]
    fn test_chunk_reset() {
        let mut chunk = Chunk::new(4096);
        
        let ptr = chunk.allocate(64).unwrap();
        assert!(!ptr.is_null());
        assert_eq!(chunk.ref_count(), 1);
        
        chunk.decrement_ref_count();
        assert!(chunk.is_empty());
        
        chunk.reset();
        assert_eq!(chunk.used(), 0);
        assert_eq!(chunk.ref_count(), 0);
    }

    #[test]
    fn test_alignment() {
        let mut chunk = Chunk::new(4096);
        
        // Allocate with various alignments
        let _ptr1 = chunk.allocate_aligned(1, 1).unwrap();
        let ptr2 = chunk.allocate_aligned(1, 16).unwrap();
        
        assert_eq!(ptr2 as usize % 16, 0);
    }

    #[test]
    fn test_contains() {
        let mut chunk = Chunk::new(4096);
        let ptr = chunk.allocate(64).unwrap();
        
        assert!(chunk.contains(ptr));
        assert!(!chunk.contains(std::ptr::null()));
    }
}
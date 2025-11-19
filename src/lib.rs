//! A chunked bump allocator with selective reset capabilities.
//! 
//! This allocator divides memory into fixed-size chunks and uses bump allocation
//! within each chunk. Each chunk tracks live allocations via reference counting,
//! enabling selective chunk reset when all allocations in a chunk are freed.

mod chunk;
mod allocator;
mod allocation;

pub use allocator::ChunkedBumpAllocator;
pub use allocation::Allocation;

// Re-export for convenience
pub use chunk::Chunk;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        let mut allocator = ChunkedBumpAllocator::new(4096);
        
        let alloc1 = allocator.allocate(64);
        let alloc2 = allocator.allocate(128);
        
        assert_ne!(alloc1.as_ptr(), alloc2.as_ptr());
        assert_eq!(allocator.chunk_count(), 1);
    }

    #[test]
    fn test_multiple_chunks() {
        let mut allocator = ChunkedBumpAllocator::new(1024);
        
        // Allocate enough to span multiple chunks
        let _alloc1 = allocator.allocate(800);
        let _alloc2 = allocator.allocate(800); // Should create new chunk
        
        assert_eq!(allocator.chunk_count(), 2);
    }

    #[test]
    fn test_selective_reset() {
        let mut allocator = ChunkedBumpAllocator::new(4096);
        
        let alloc1 = allocator.allocate(64);
        let alloc2 = allocator.allocate(128);
        
        // Drop alloc1, its chunk should become eligible for reset
        drop(alloc1);
        
        let reset_count = allocator.reset_empty_chunks();
        
        // Should not reset because alloc2 is still alive
        assert_eq!(reset_count, 0);
        
        drop(alloc2);
        let reset_count = allocator.reset_empty_chunks();
        
        // Now should reset
        assert_eq!(reset_count, 1);
    }

    #[test]
    fn test_alignment() {
        let mut allocator = ChunkedBumpAllocator::new(4096);
        
        let alloc = allocator.allocate_aligned(17, 16);
        let addr = alloc.as_ptr() as usize;
        
        assert_eq!(addr % 16, 0, "Pointer should be 16-byte aligned");
    }
}
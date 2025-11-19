use crate::chunk::Chunk;
use std::ptr::NonNull;

/// An RAII guard for an allocation.
/// 
/// When dropped, automatically decrements the reference count
/// of the chunk it came from.
pub struct Allocation{
    ptr: NonNull<u8>,
    size: usize,
    chunk: *const Chunk,
}

impl Allocation{
    /// Creates a new allocation guard.
    /// 
    /// # Safety
    /// 
    /// The chunk pointer must remain valid for the lifetime of this Allocation.
    /// The caller (allocator) must ensure chunks are not freed while
    /// allocations from them exist.
    pub(crate) unsafe fn new(ptr: *mut u8, size: usize, chunk: *const Chunk) -> Self {
        Allocation {
            ptr: NonNull::new(ptr).expect("Allocation pointer cannot be null"),
            size,
            chunk,

        }
    }
    
    /// Returns the raw pointer to the allocation.
    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }
    
    /// Returns the size of this allocation.
    pub fn size(&self) -> usize {
        self.size
    }
    
    /// Returns a mutable slice view of the allocation.
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size)
        }
    }
    
    /// Returns an immutable slice view of the allocation.
    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self.ptr.as_ptr(), self.size)
        }
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        // Decrement the reference count of the chunk
        unsafe {
            (*self.chunk).decrement_ref_count();
        }
    }
}

// Allocations cannot be sent between threads (they're tied to the allocator)
// This is safe because the allocator is not thread-safe by default

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::Chunk;

    #[test]
    fn test_allocation_drop() {
        let mut chunk = Chunk::new(4096);
        let ptr = chunk.allocate(64).unwrap();
        
        assert_eq!(chunk.ref_count(), 1);
        
        {
            let _alloc = unsafe {
                Allocation::new(ptr, 64, &chunk as *const Chunk)
            };
            assert_eq!(chunk.ref_count(), 1);
        } // alloc dropped here
        
        assert_eq!(chunk.ref_count(), 0);
    }

    #[test]
    fn test_allocation_slice() {
        let mut chunk = Chunk::new(4096);
        let ptr = chunk.allocate(64).unwrap();
        
        let mut alloc = unsafe {
            Allocation::new(ptr, 64, &chunk as *const Chunk)
        };
        
        // Write to the allocation
        let slice = alloc.as_slice_mut();
        slice[0] = 42;
        slice[1] = 84;
        
        // Read back
        assert_eq!(alloc.as_slice()[0], 42);
        assert_eq!(alloc.as_slice()[1], 84);
    }
}
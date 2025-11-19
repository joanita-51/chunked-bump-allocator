use chunked_bump_allocator::ChunkedBumpAllocator;

fn main() {
    // Create allocator with 4KB chunks
    let mut allocator = ChunkedBumpAllocator::new(4096);
    
    // Allocate memory
    let mut alloc = allocator.allocate(128);
    alloc.as_slice_mut().fill(42);
    
    // Automatically freed when dropped
    drop(alloc);
    
    // Reset empty chunks
    allocator.reset_empty_chunks();
}
use chunked_bump_allocator::ChunkedBumpAllocator;

fn main() {
    // Create allocator with 4KB chunks
    let mut allocator = ChunkedBumpAllocator::new(4096);
    
    println!("=== Basic Usage Example ===\n");
    
    // Allocate some memory
    println!("Allocating 3 objects...");
    let mut alloc1 = allocator.allocate(64);
    let mut alloc2 = allocator.allocate(128);
    let alloc3 = allocator.allocate(256);
    
    // Write to allocations
    alloc1.as_slice_mut().fill(1);
    alloc2.as_slice_mut().fill(2);
    
    println!("Stats after allocation: {}", allocator.stats());
    
    // Drop some allocations
    println!("\nDropping first two allocations...");
    drop(alloc1);
    drop(alloc2);
    
    println!("Stats after drops: {}", allocator.stats());
    
    // Try to reset
    println!("\nAttempting to reset empty chunks...");
    let reset = allocator.reset_empty_chunks();
    println!("Chunks reset: {} (should be 0, alloc3 still alive)", reset);
    
    // Drop last allocation
    println!("\nDropping last allocation...");
    drop(alloc3);
    
    println!("Stats after all drops: {}", allocator.stats());
    
    // Now reset should work
    println!("\nAttempting to reset empty chunks...");
    let reset = allocator.reset_empty_chunks();
    println!("Chunks reset: {} (should be 1)", reset);
    
    println!("\nFinal stats: {}", allocator.stats());
    
    // Demonstrate multiple chunks
    println!("\n=== Multiple Chunks Example ===\n");
    let mut allocator2 = ChunkedBumpAllocator::new(1024);
    
    let _a1 = allocator2.allocate(800);
    let _a2 = allocator2.allocate(800); // New chunk
    let _a3 = allocator2.allocate(800); // Another chunk
    
    println!("Allocated 3x800 bytes with 1KB chunks");
    println!("Stats: {}", allocator2.stats());
}
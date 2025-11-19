use chunked_bump_allocator::ChunkedBumpAllocator;

#[test]
fn test_mixed_lifetime_allocations() {
    let mut allocator = ChunkedBumpAllocator::new(4096);
    
    // Simulate mixed lifetimes
    let long_lived = allocator.allocate(1000);
    
    // Many short-lived allocations
    for _ in 0..10 {
        let short_lived = allocator.allocate(100);
        drop(short_lived); // Immediately dropped
    }
    
    // Should not have reset while long_lived exists
    let reset = allocator.reset_empty_chunks();
    assert_eq!(reset, 0);
    
    drop(long_lived);
    
    // Now can reset
    let reset = allocator.reset_empty_chunks();
    assert!(reset > 0);
}

#[test]
fn test_memory_reuse_after_reset() {
    let mut allocator = ChunkedBumpAllocator::new(4096);
    
    // First allocation
    let alloc1 = allocator.allocate(1000);
    let ptr1 = alloc1.as_ptr();
    drop(alloc1);
    
    allocator.reset_empty_chunks();
    
    // Second allocation after reset - might reuse same chunk
    let alloc2 = allocator.allocate(1000);
    let ptr2 = alloc2.as_ptr();
    
    // Should have reused the chunk (same starting address)
    assert_eq!(ptr1, ptr2);
    assert_eq!(allocator.chunk_count(), 1);
}

#[test]
fn test_fragmentation_scenario() {
    let mut allocator = ChunkedBumpAllocator::new(4096);
    
    // Allocate multiple objects in same chunk
    let allocs: Vec<_> = (0..10)
        .map(|_| allocator.allocate(200))
        .collect();
    
    assert_eq!(allocator.chunk_count(), 1);
    
    // Free every other allocation (create fragmentation)
    let mut kept = Vec::new();
    for (i, alloc) in allocs.into_iter().enumerate() {
        if i % 2 == 0 {
            kept.push(alloc);
        }
        // else dropped
    }
    
    // Cannot reset because some allocations still live
    let reset = allocator.reset_empty_chunks();
    assert_eq!(reset, 0);
    
    // Still shows fragmentation
    let stats = allocator.stats();
    assert!(stats.overall_utilization < 0.6); // Less than if fully packed
}

#[test]
fn test_large_number_of_allocations() {
    let mut allocator = ChunkedBumpAllocator::new(4096);
    let mut allocations = Vec::new();
    
    // Create 1000 small allocations
    for _ in 0..1000 {
        allocations.push(allocator.allocate(32));
    }
    
    let stats = allocator.stats();
    assert!(stats.chunk_count >= 8); // At least ~32KB total
    assert_eq!(stats.total_live_allocations, 1000);
    
    // Drop all
    allocations.clear();
    
    // Reset all
    let reset = allocator.reset_empty_chunks();
    assert_eq!(reset, stats.chunk_count);
}
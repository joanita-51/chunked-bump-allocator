use chunked_bump_allocator::ChunkedBumpAllocator;
use bumpalo::Bump;

fn test_reset_policies() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  RESET POLICY COMPARISON");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    let long_lived = 100;
    let short_lived = 900;
    let alloc_size = 64;
    
    // Test keeping 0, 1, 2 spare chunks
    for keep_spare in [0, 1, 2] {
        let mut allocator = ChunkedBumpAllocator::new(8192);
        let mut long_allocs = Vec::new();
        
        // Allocate long-lived
        for _ in 0..long_lived {
            long_allocs.push(allocator.allocate(alloc_size));
        }
        
        // Allocate and drop short-lived
        for _ in 0..short_lived {
            drop(allocator.allocate(alloc_size));
        }
        
        // Reset with different policies
        let removed = allocator.reset_empty_chunks_keep_spare(keep_spare);
        
        let stats = allocator.stats();
        let live_kb = (long_lived * alloc_size) as f32 / 1024.0;
        let peak_kb = stats.total_capacity as f32 / 1024.0;
        let savings = ((63.06 - peak_kb) / 63.06) * 100.0;
        
        println!("Keep {} spare chunk(s):", keep_spare);
        println!("  Chunks remaining: {}", stats.chunk_count);
        println!("  Peak memory:      {:.2} KB", peak_kb);
        println!("  Live data:        {:.2} KB", live_kb);
        println!("  Efficiency:       {:.1}%", (live_kb / peak_kb) * 100.0);
        println!("  vs bumpalo:       {:.1}% savings\n", savings);
    }
}

fn main() {
    println!("==============================================");
    println!("  Memory Usage Comparison: Mixed Lifetimes");
    println!("==============================================\n");
    
    // Scenario: 10% long-lived, 90% short-lived
    // This is where your allocator should shine
    
    let total_allocs = 1000;
    let long_lived_count = 100;
    let short_lived_count = 900;
    let alloc_size = 64;
    
    println!("Scenario:");
    println!("  - {} total allocations of {} bytes each", total_allocs, alloc_size);
    println!("  - {} long-lived (10%)", long_lived_count);
    println!("  - {} short-lived (90%) - freed immediately\n", short_lived_count);
    
    // Test 1: Chunked Bump Allocator
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  CHUNKED BUMP ALLOCATOR");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    {
        let mut allocator = ChunkedBumpAllocator::new(8192);
        let mut long_lived = Vec::new();
        
        // Phase 1: Allocate long-lived
        for _i in 0..long_lived_count {
            long_lived.push(allocator.allocate(alloc_size));
        }
        
        println!("After long-lived allocations:");
        print_stats(&allocator);
        
        // Phase 2: Allocate short-lived (then immediately drop)
        for _i in 0..short_lived_count {
            let temp = allocator.allocate(alloc_size);
            drop(temp);
        }
        
        println!("\nAfter short-lived allocations (dropped):");
        print_stats(&allocator);
        
        // Phase 3: Reset empty chunks
        let reset_count = allocator.reset_empty_chunks();
        
        println!("\nAfter reset ({} chunks reset):", reset_count);
        print_stats(&allocator);
        
        // Calculate memory efficiency
        let live_data = (long_lived_count * alloc_size) as f32 / 1024.0;
        let peak_memory = allocator.total_capacity() as f32 / 1024.0;
        let efficiency = (live_data / peak_memory) * 100.0;
        
        println!("\n  Memory Efficiency:");
        println!("    Live data:    {:.2} KB", live_data);
        println!("    Peak memory:  {:.2} KB", peak_memory);
        println!("    Efficiency:   {:.1}%", efficiency);
        println!("    Waste:        {:.2} KB", peak_memory - live_data);
    }
    
    // Test 2: bumpalo
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  BUMPALO");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    {
        let mut bump = Bump::new();
        let mut long_lived = Vec::new();
        
        // Phase 1: Allocate long-lived
        for i in 0..long_lived_count {
            long_lived.push(bump.alloc([i as u8; 64]));
        }
        
        let after_long = bump.allocated_bytes();
        println!("After long-lived allocations:");
        println!("  Allocated: {:.2} KB", after_long as f32 / 1024.0);
        
        // Phase 2: Allocate short-lived
        let mut short_lived = Vec::new();
        for i in 0..short_lived_count {
            short_lived.push(bump.alloc([i as u8; 64]));
        }
        
        let after_short = bump.allocated_bytes();
        println!("\nAfter short-lived allocations:");
        println!("  Allocated: {:.2} KB", after_short as f32 / 1024.0);
        
        // Phase 3: Drop short-lived references
        short_lived.clear();
        
        let after_drop = bump.allocated_bytes();
        println!("\nAfter dropping short-lived references:");
        println!("  Allocated: {:.2} KB (UNCHANGED - memory not reclaimed!)", after_drop as f32 / 1024.0);
        
        // Calculate memory efficiency
        let live_data = (long_lived_count * alloc_size) as f32 / 1024.0;
        let peak_memory = after_drop as f32 / 1024.0;
        let efficiency = (live_data / peak_memory) * 100.0;
        
        println!("\n  Memory Efficiency:");
        println!("    Live data:    {:.2} KB", live_data);
        println!("    Peak memory:  {:.2} KB", peak_memory);
        println!("    Efficiency:   {:.1}%", efficiency);
        println!("    Waste:        {:.2} KB", peak_memory - live_data);
        println!("\n  Note: bumpalo CANNOT reclaim memory until full reset");
    }
    
    // Comparison summary
    println!("\n==============================================");
    println!("  SUMMARY");
    println!("==============================================\n");
    
    // Re-run for final comparison
    let chunked_peak = {
        let mut allocator = ChunkedBumpAllocator::new(8192);
        let mut long_lived = Vec::new();
        for _i in 0..long_lived_count {
            long_lived.push(allocator.allocate(alloc_size));
        }
        for _i in 0..short_lived_count {
            drop(allocator.allocate(alloc_size));
        }
        allocator.reset_empty_chunks();
        allocator.total_capacity()
    };
    
    let bumpalo_peak = {
        let mut bump = Bump::new();
        let mut long_lived = Vec::new();
        for i in 0..long_lived_count {
            long_lived.push(bump.alloc([i as u8; 64]));
        }
        for i in 0..short_lived_count {
            let _ = bump.alloc([i as u8; 64]);
        }
        bump.allocated_bytes()
    };
    
    let live_data_kb = (long_lived_count * alloc_size) as f32 / 1024.0;
    let chunked_kb = chunked_peak as f32 / 1024.0;
    let bumpalo_kb = bumpalo_peak as f32 / 1024.0;
    
    let memory_savings = ((bumpalo_kb - chunked_kb) / bumpalo_kb) * 100.0;
    
    println!("Live data needed:       {:.2} KB", live_data_kb);
    println!("Chunked allocator:      {:.2} KB", chunked_kb);
    println!("bumpalo:                {:.2} KB", bumpalo_kb);
    println!("\n>>> Memory savings:     {:.1}% <<<", memory_savings);
    println!(">>> Overhead reduction: {:.2} KB <<<", bumpalo_kb - chunked_kb);

    test_reset_policies();
}

fn print_stats(allocator: &ChunkedBumpAllocator) {
    let stats = allocator.stats();
    println!("  Chunks:           {}", stats.chunk_count);
    println!("  Empty chunks:     {}", stats.empty_chunks);
    println!("  Total capacity:   {:.2} KB", stats.total_capacity as f32 / 1024.0);
    println!("  Total used:       {:.2} KB", stats.total_used as f32 / 1024.0);
    println!("  Utilization:      {:.1}%", stats.overall_utilization * 100.0);
    println!("  Live allocations: {}", stats.total_live_allocations);
}
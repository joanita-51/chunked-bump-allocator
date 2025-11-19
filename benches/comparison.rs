use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use chunked_bump_allocator::ChunkedBumpAllocator;
use bumpalo::Bump;

// Test structure
#[derive(Clone, Copy)]
struct TestData {
    id: u64,
    value: u64,
}

/// Scenario 1: All allocations have same lifetime
/// This favors bumpalo (its design case)
fn bench_same_lifetime(c: &mut Criterion) {
    let mut group = c.benchmark_group("same_lifetime");
    
    // Test with different allocation counts
    for count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("chunked", count),
            count,
            |b, &count| {
                b.iter(|| {
                    let mut allocator = ChunkedBumpAllocator::new(8192);
                    let mut allocations = Vec::new();
                    
                    // Allocate everything
                    for i in 0..count {
                        let alloc = allocator.allocate(16); // Removed mut
                        let data = TestData { id: i, value: i * 2 };
                        unsafe {
                            std::ptr::write(alloc.as_ptr() as *mut TestData, data);
                        }
                        allocations.push(alloc);
                    }
                    
                    // Use the data
                    black_box(&allocations);
                    
                    // Drop all at once
                    allocations.clear();
                    allocator.reset_empty_chunks();
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("bumpalo", count),
            count,
            |b, &count| {
                b.iter(|| {
                    let mut bump = Bump::new(); // Added mut
                    let mut data_vec = Vec::new();
                    
                    // Allocate everything
                    for i in 0..count {
                        let data = bump.alloc(TestData { id: i, value: i * 2 });
                        data_vec.push(data);
                    }
                    
                    // Use the data
                    black_box(&data_vec);
                    
                    // Reset all at once
                    bump.reset();
                });
            },
        );
    }
    
    group.finish();
}

/// Scenario 2: Mixed lifetimes - SHORT-LIVED allocations
/// This favors your allocator (can reclaim memory early)
fn bench_mixed_lifetime_short_lived(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_lifetime_short");
    
    // 90% short-lived, 10% long-lived
    for total in [1000, 10000].iter() {
        let short_lived = (total * 9) / 10;
        let long_lived = total - short_lived;
        
        group.bench_with_input(
            BenchmarkId::new("chunked", total),
            total,
            |b, _| {
                b.iter(|| {
                    let mut allocator = ChunkedBumpAllocator::new(8192);
                    let mut long_allocs = Vec::new();
                    
                    // Allocate long-lived first
                    for i in 0..long_lived {
                        let alloc = allocator.allocate(16); // Removed mut
                        let data = TestData { id: i, value: i * 2 };
                        unsafe {
                            std::ptr::write(alloc.as_ptr() as *mut TestData, data);
                        }
                        long_allocs.push(alloc);
                    }
                    
                    // Allocate and immediately free short-lived
                    for i in 0..short_lived {
                        let alloc = allocator.allocate(16); // Removed mut
                        let data = TestData { id: i + long_lived, value: i * 2 };
                        unsafe {
                            std::ptr::write(alloc.as_ptr() as *mut TestData, data);
                        }
                        drop(alloc); // Freed immediately!
                    }
                    
                    // Reset empty chunks (YOUR ADVANTAGE!)
                    let reset = allocator.reset_empty_chunks();
                    black_box(reset);
                    
                    // Long-lived still valid
                    black_box(&long_allocs);
                    
                    // Cleanup
                    long_allocs.clear();
                    allocator.reset_empty_chunks();
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("bumpalo", total),
            total,
            |b, _| {
                b.iter(|| {
                    let mut bump = Bump::new(); // Added mut
                    let mut long_data = Vec::new();
                    
                    // Allocate long-lived
                    for i in 0..long_lived {
                        let data = bump.alloc(TestData { id: i, value: i * 2 });
                        long_data.push(data);
                    }
                    
                    // Allocate short-lived
                    let mut short_data = Vec::new();
                    for i in 0..short_lived {
                        let data = bump.alloc(TestData { 
                            id: i + long_lived, 
                            value: i * 2 
                        });
                        short_data.push(data);
                    }
                    
                    // Drop short-lived references
                    short_data.clear();
                    
                    // CANNOT reclaim memory! (bumpalo limitation)
                    // Memory stays allocated until full reset
                    
                    black_box(&long_data);
                    
                    // Full reset at end
                    bump.reset();
                });
            },
        );
    }
    
    group.finish();
}

/// Scenario 3: Interleaved allocations and frees
/// Simulates real workload with mixed lifetimes
fn bench_interleaved(c: &mut Criterion) {
    c.bench_function("interleaved_chunked", |b| {
        b.iter(|| {
            let mut allocator = ChunkedBumpAllocator::new(8192);
            let mut active = Vec::new();
            
            for i in 0..1000 {
                // Allocate
                let alloc = allocator.allocate(32); // Removed mut
                let data = TestData { id: i, value: i * 2 };
                unsafe {
                    std::ptr::write(alloc.as_ptr() as *mut TestData, data);
                }
                active.push(alloc);
                
                // Every 10 iterations, free some old allocations
                if i % 10 == 0 && active.len() > 5 {
                    active.drain(0..5); // Free first 5
                    allocator.reset_empty_chunks(); // Reclaim memory
                }
            }
            
            black_box(&active);
        });
    });
    
    c.bench_function("interleaved_bumpalo", |b| {
        b.iter(|| {
            let mut bump = Bump::new(); // Added mut
            let mut active = Vec::new();
            
            for i in 0..1000 {
                // Allocate
                let data = bump.alloc(TestData { id: i, value: i * 2 });
                active.push(data);
                
                // Every 10 iterations, drop references
                if i % 10 == 0 && active.len() > 5 {
                    active.drain(0..5); // Drop references
                    // CANNOT reclaim memory!
                }
            }
            
            black_box(&active);
            bump.reset(); // Only at the end
        });
    });
}

/// Scenario 4: Peak memory usage comparison
/// This shows memory efficiency difference
fn bench_peak_memory(c: &mut Criterion) {
    c.bench_function("peak_memory_chunked", |b| {
        b.iter(|| {
            let mut allocator = ChunkedBumpAllocator::new(8192);
            let mut keep = Vec::new();
            
            // Allocate 100 long-lived
            for _i in 0..100 {
                keep.push(allocator.allocate(64));
            }
            
            // Allocate and free 900 short-lived
            for _i in 0..900 {
                let alloc = allocator.allocate(64);
                drop(alloc);
            }
            
            // Reclaim short-lived memory
            allocator.reset_empty_chunks();
            
            // Report stats
            let stats = allocator.stats();
            black_box(stats.total_capacity); // Peak memory
            
            black_box(&keep);
        });
    });
    
    c.bench_function("peak_memory_bumpalo", |b| {
        b.iter(|| {
            let mut bump = Bump::new(); // Added mut
            let mut keep = Vec::new();
            
            // Allocate 100 long-lived
            for i in 0..100 {
                keep.push(bump.alloc(TestData { id: i, value: i }));
            }
            
            // Allocate 900 short-lived
            for i in 0..900 {
                let _ = bump.alloc(TestData { id: i + 100, value: i });
            }
            
            // CANNOT reclaim short-lived memory!
            // Peak memory = all 1000 allocations
            
            black_box(&keep);
            bump.reset(); // Reset at end
        });
    });
}

criterion_group!(
    benches,
    bench_same_lifetime,
    bench_mixed_lifetime_short_lived,
    bench_interleaved,
    bench_peak_memory
);
criterion_main!(benches);
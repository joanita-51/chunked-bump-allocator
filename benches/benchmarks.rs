use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use chunked_bump_allocator::ChunkedBumpAllocator;

fn bench_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation");
    
    for size in [64, 256, 1024, 4096].iter() {
        group.bench_with_input(BenchmarkId::new("allocate", size), size, |b, &size| {
            b.iter(|| {
                let mut allocator = ChunkedBumpAllocator::new(8192);
                for _ in 0..100 {
                    let alloc = allocator.allocate(size);
                    black_box(alloc);
                }
            });
        });
    }
    
    group.finish();
}

fn bench_allocation_with_reset(c: &mut Criterion) {
    c.bench_function("allocate_and_reset", |b| {
        b.iter(|| {
            let mut allocator = ChunkedBumpAllocator::new(8192);
            
            // Allocate
            let allocs: Vec<_> = (0..100)
                .map(|_| allocator.allocate(64))
                .collect();
            
            // Drop
            drop(allocs);
            
            // Reset
            black_box(allocator.reset_empty_chunks());
        });
    });
}

fn bench_mixed_lifetimes(c: &mut Criterion) {
    c.bench_function("mixed_lifetimes", |b| {
        b.iter(|| {
            let mut allocator = ChunkedBumpAllocator::new(8192);
            
            // Long-lived allocation
            let _long = allocator.allocate(1000);
            
            // Many short-lived
            for _ in 0..50 {
                let short = allocator.allocate(100);
                black_box(short);
            }
        });
    });
}

fn bench_vs_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison");
    
    group.bench_function("chunked_allocator", |b| {
        b.iter(|| {
            let mut allocator = ChunkedBumpAllocator::new(8192);
            let mut allocs = Vec::new();
            
            for _ in 0..1000 {
                allocs.push(allocator.allocate(32));
            }
            
            black_box(allocs);
        });
    });
    
    group.bench_function("vec_of_vecs", |b| {
        b.iter(|| {
            let mut vecs: Vec<Vec<u8>> = Vec::new();
            
            for _ in 0..1000 {
                vecs.push(vec![0u8; 32]);
            }
            
            black_box(vecs);
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_allocation,
    bench_allocation_with_reset,
    bench_mixed_lifetimes,
    bench_vs_vec
);
criterion_main!(benches);
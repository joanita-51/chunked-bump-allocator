# Chunked Bump Allocator with Selective Reset

Course project at UNIST.

**Author:** Joanita Nakityo

## Overview

This project implements a Rust bump allocator that supports selective
reset of individual memory chunks.

Traditional arena-wide reset requires all allocations in an arena
to become unused before its memory can be reclaimed. This can retain
unneeded memory when short-lived and long-lived allocations coexist.

This allocator divides memory into independent chunks and tracks
the live allocations in each chunk. An empty chunk can be reset
and reused while other chunks retain their live allocations.

## Design

- **Chunked allocation:** Memory is organized into fixed-size chunks,
  each with its own bump pointer and allocation metadata.
- **Live-allocation tracking:** A per-chunk reference count tracks
  outstanding allocations.
- **RAII guards:** Dropping an allocation guard updates the count
  for its originating chunk.
- **Selective reset:** A previously used chunk becomes eligible
  for reset when its live-allocation count reaches zero.

Individual dropped allocations do not immediately make their space
reusable. Reuse occurs when the entire originating chunk is empty
and reset.

The evaluated configuration uses 8 KB chunks.

## Evaluation

The project evaluates synthetic, single-threaded workloads with:

- 1,000 allocations of 64 bytes each.
- Short-lived to long-lived allocation ratios of 90/10, 70/30,
  and 50/50.
- Comparisons against bumpalo and Rust's default allocator.
- Measurements of allocation time and memory utilization.

The experiments explore the trade-off between memory reclamation
and allocation speed. Selective reset improves memory reuse in
the evaluated workloads, while reference counting and chunk
management add overhead.

The report records average allocation times of approximately
19 µs for this allocator and 3.7 µs for bumpalo under its benchmark
conditions. These results describe the evaluated synthetic workloads
and are not general performance guarantees.

## Limitations

- The current implementation targets single-threaded use.
- Chunk size is fixed at 8 KB in the evaluated configuration.
- Allocations larger than a chunk are not currently supported.
- A live allocation prevents its chunk from being reset.
- Evaluation uses synthetic workloads; real-application performance
  remains to be studied.

## Future Work

- Per-thread chunk pools and multithreaded support.
- Adaptive chunk sizing.
- Support for allocations larger than a chunk.
- Evaluation using real applications.

## Project Report

See [the project report](Chunked_Bump_Allocator_Report.pdf)
for the design, experiments, and discussion.

## References

- Grossman et al. Region-Based Memory Management in Cyclone. 2002.
- Leijen et al. Mimalloc: Free List Sharding in Action. 2019.
- Matsakis and Klock. The Rust Language. 2014.

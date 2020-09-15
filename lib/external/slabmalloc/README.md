# slabmalloc [![Build Status](https://travis-ci.org/gz/rust-slabmalloc.svg)](https://travis-ci.org/gz/rust-slabmalloc) [![Crates.io](https://img.shields.io/crates/v/slabmalloc.svg)](https://crates.io/crates/slabmalloc)

Simple slab based malloc implementation in rust, in order to provide the
necessary interface to rusts liballoc library. slabmalloc only relies on
libcore and is designed to be used in kernel level code as the only interface a
client needs to provide is the necessary mechanism to allocate and free 4KiB
frames (or any other default page-size on non-x86 hardware).


## Build

By default this library should compile using `cargo build` with nightly versions of the
Rust compiler.

Add the following line to the Cargo.toml dependencies:
```
slabmalloc = ...
```

Due to the use of [`const_fn`](https://github.com/rust-lang/rust/issues/57563),
if you use the library with a stable rustc the `unstable` feature needs to be
disabled:

```
slabmalloc = { version = ..., default_features = false }
```

## Documentation
* [API Documentation](https://docs.rs/slabmalloc)
* [Examples](examples/global_alloc.rs)

## API Usage

slabmalloc has two main components described here. However, if you just want to
implement a GlobalAlloc trait have a look at the provided
[example](examples/global_alloc.rs).

It provides a ZoneAllocator to allocate arbitrary sized objects:
```rust
let object_size = 12;
let alignment = 4;
let layout = Layout::from_size_align(object_size, alignment).unwrap();

// We need something that can provide backing memory
// (4 KiB and 2 MiB pages) to our ZoneAllocator
// (see tests.rs for a dummy implementation).
let mut pager = Pager::new();
let page = pager.allocate_page().expect("Can't allocate a page");

let mut zone: ZoneAllocator = Default::default();
// Prematurely fill the ZoneAllocator with memory.
// Alternatively, the allocate call would return an
// error which we can capture to refill on-demand.
unsafe { zone.refill(layout, page)? };

let allocated = zone.allocate(layout)?;
zone.deallocate(allocated, layout)?;
```

And a SCAllocator to allocate fixed sized objects:
```rust
let object_size = 10;
let alignment = 8;
let layout = Layout::from_size_align(object_size, alignment).unwrap();

// We need something that can provide backing memory
// (4 KiB and 2 MiB pages) to our ZoneAllocator
// (see tests.rs for a dummy implementation).
let mut pager = Pager::new();
let page = pager.allocate_page().expect("Can't allocate a page");

let mut sa: SCAllocator<ObjectPage> = SCAllocator::new(object_size);
// Prematurely fill the SCAllocator with memory.
// Alternatively, the allocate call would return an
// error which we can capture to refill on-demand.
unsafe { sa.refill(page) };

sa.allocate(layout)?;
```

## Performance

slabmalloc is optimized for single-threaded, fixed-size object allocations. For
anything else it will probably perform poorly (for example if your workload
does lots of reallocations, or if the allocator needs to scale to many cores).

At least on my system, it outperforms jemalloc in (silly) benchmarks:
```
test tests::jemalloc_allocate_deallocate       ... bench:          76 ns/iter (+/- 5)
test tests::jemalloc_allocate_deallocate_big   ... bench:         119 ns/iter (+/- 24)
test tests::slabmalloc_allocate_deallocate     ... bench:          38 ns/iter (+/- 8)
test tests::slabmalloc_allocate_deallocate_big ... bench:          38 ns/iter (+/- 11)
```
## On Naming

We call our allocator slabmalloc; however the name can be confusing as
slabmalloc differs a bit from the [seminal paper by Jeff
Bonwick](https://dl.acm.org/citation.cfm?id=1267263) describing the "slab
allocator". slabmalloc really is just a malloc implementation with size classes
and different allocators per class (a segregated-storage allocator), while
incorporating some of the simple and effective ideas from slab allocation.

Some notable differences for folks familiar with the slab allocator:

* The slab allocator constructor asks for an object constructor and destructor
function to initialize/deinitialize objects. slabmalloc is more malloc-like;
it just deals with memory, not object caching.

* A slab in the slab allocator consists of one or more pages of virtually
contiguous memory, carved up into equal-size chunks, with a reference count
indicating how many of those chunks have been allocated. Instead, slabmalloc
uses a (cache-line sized) bitmap to track objects within a slab. Similarly, the
slab allocator builds a linked-list of free objects, whereas slabmalloc scans the
bitmap in a slab to find a free slot.

* For large objects, the slab allocator does not embed meta-data within the
slab page. Because, you could fit only one 2 KiB buffer on a 4 KiB page with
the embedded slab data. Moreover, with large (multi-page) slabs it can not
determine the slab data address from the buffer address. So a per-cache
hash-table is used to map allocated objects to meta-data memory. In slabmalloc,
the meta-data is always at the end of the page. It uses different slab sizes to
ensure bigger objects are not allocated on small slab-pages. The problem of
determining the slab base address is alleviated in rust as we also receive the
object size on deallocations (we determine the size of the underlying slab by
looking at the size of the object that is to be freed).
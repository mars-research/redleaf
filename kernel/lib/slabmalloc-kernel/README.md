# slabmalloc [![Build Status](https://travis-ci.org/gz/rust-slabmalloc.svg)](https://travis-ci.org/gz/rust-slabmalloc) [![Crates.io](https://img.shields.io/crates/v/slabmalloc.svg)](https://crates.io/crates/slabmalloc)

Simple slab based malloc implementation in rust, in order to provide the
necessary interface to rusts liballoc library. slabmalloc only relies on libcore
and is designed to be used in kernel level code as the only interface a client
needs to provide is the  necessary mechanism to allocate and free 4KiB frames
(or any other default page-size on non-x86 hardware).

## Usage

* The slabmalloc API is designed to satisfy the rust liballoc low-level memory allocation interface:

```rust
use slabmalloc::{SafeZoneAllocator};
#[global_allocator]
static MEM_PROVIDER: SafeZoneAllocator = SafeZoneAllocator::new(&PAGER);
```

* Use the ZoneAllocator to allocate arbitrary sized objects:
```rust
let object_size = 12;
let alignment = 4;
let mmap = Mutex::new(MmapPageProvider::new());
let mut zone = ZoneAllocator::new(&mmap);

unsafe {
  let layout = Layout::from_size_align(object_size, alignment).unwrap();
  let allocated = zone.allocate(layout);
  assert!(!allocated.is_null());
  zone.deallocate(allocated, layout);
}
```

* Use the SCAllocator to allocate fixed sized objects:
```rust
let object_size = 10;
let alignment = 8;
let layout = Layout::from_size_align(object_size, alignment).unwrap();
let mut mmap = Mutex::new(MmapPageProvider::new());
let mut sa: SCAllocator = SCAllocator::new(object_size, &mut mmap);
sa.allocate(layout);
```

## Using on stable
By default this packages requires a nightly version of the Rust
compiler. To be able to use this package with a stable version of the
Rust compiler, default features have to be disabled, e.g. with
```
slabmalloc = { version = ..., default_features = false }
```

## Documentation
* [API Documentation](https://docs.rs/slabmalloc)

## TODO
* No focus on performance yet

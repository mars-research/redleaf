[![Build Status](https://travis-ci.org/gz/rust-elfloader.svg?branch=master)](https://travis-ci.org/gz/rust-elfloader)

# elfloader

A library to load and relocate ELF files in memory. This library depends only on
libcore so it can be used in kernel level code, for example to
load user-space programs.

## How-to use
Clients will have to implement the ElfLoader trait:

```rust
/// A simple ExampleLoader, that implements ElfLoader
/// but does nothing but logging
struct ExampleLoader {
    vbase: u64,
}

impl ElfLoader for ExampleLoader {
    fn allocate(&mut self, load_headers: LoadableHeaders) -> Result<(), &'static str> {
        for header in load_headers {
            info!(
                "allocate base = {:#x} size = {:#x} flags = {}",
                header.virtual_addr(),
                header.mem_size(),
                header.flags()
            );
        }
        Ok(())
    }

    fn relocate(&mut self, entry: &Rela<P64>) -> Result<(), &'static str> {
        let typ = TypeRela64::from(entry.get_type());
        let addr: *mut u64 = (self.vbase + entry.get_offset()) as *mut u64;

        match typ {
            TypeRela64::R_RELATIVE => {
                // This is a relative relocation, add the offset (where we put our
                // binary in the vspace) to the addend and we're done.
                info!(
                    "R_RELATIVE *{:p} = {:#x}",
                    addr,
                    self.vbase + entry.get_addend()
                );
                Ok(())
            }
            _ => Err("Unexpected relocation encountered"),
        }
    }

    fn load(&mut self, base: VAddr, region: &[u8]) -> Result<(), &'static str> {
        let start = self.vbase + base;
        let end = self.vbase + base + region.len();
        info!("load region into = {:#x} -- {:#x}", start, end);
        Ok(())
    }
}
```

Then, with ElfBinary, a ELF file is loaded using `load`:

```rust
let binary_blob = fs::read("test/test").expect("Can't read binary");
let binary = ElfBinary::new("test", binary_blob.as_slice()).expect("Got proper ELF file");
let mut loader = ExampleLoader::new(0x1000_0000);
binary.load(&mut loader).expect("Can't load the binary?");
```

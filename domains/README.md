# RedLeaf Domains

This directory contains a set of domains to be used on RedLeaf. All RedLeaf domains are build distributed as standalone ELF files.

## `x86_64-unknown-redleaf.json`

`x86_64-unknown-redleaf` is the canonical target for building domains for RedLeaf. Domain ELF files must have the `trusted_entry` symbol.

## Repo structure revamp

Note for maintainers: All Makefiles inside domain directories are unused.

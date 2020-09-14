# RedSys

RedSys provides a safe layer for drivers to access system resources.

## Resource Request Declaration

A driver must implement the `Driver` Trait, which requires the driver to define a constant describing all resources it wishes to access.

## System Resources

### `RawMemoryRegion`

Raw access to a region of physical memory.

Example drivers:
- `vgatext`
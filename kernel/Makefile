FEATURES =
#FEATURES += --features "trace_alloc"
#FEATURES += --features "smp"
FEATURES += --features "trace_vspace"
FEATURES += --features "page_fault_on_ist"
#FEATURES += --features "trace_sched"
#FEATURES += --features "test_shadow"

ifeq ($(LARGE_MEM),true)
FEATURES += --features "large_mem"
MEM = -m 10240M
else
MEM = -m 2048M
endif

ifeq ($(IXGBE),true)
FEATURES += --features "c220g2_ixgbe"
PCI_FEATURES += --features "c220g2_ixgbe"
endif

export PCI_FEATURES

.PHONY: build/libredleaf.a
build/libredleaf.a:
	@BUILD_VERSION="$(shell date) ($(shell whoami)@$(shell hostname))" RUST_TARGET_PATH="$(shell pwd)" RUSTFLAGS="-Z emit-stack-sizes" cargo build --out-dir=build -Z build-std=core,alloc -Z features=host_dep -Z unstable-options $(FEATURES)

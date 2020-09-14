.PHONY: domains
domains:
	@BUILD_VERSION="$(shell date) ($(shell whoami)@$(shell hostname))" RUST_TARGET_PATH="$(shell pwd)" RUSTFLAGS="-Z emit-stack-sizes" cargo build --out-dir=build -Z build-std=core,alloc -Z features=host_dep -Z unstable-options $(FEATURES)

domain/%:
	@cd $* && BUILD_VERSION="$(shell date) ($(shell whoami)@$(shell hostname))" RUST_TARGET_PATH="$(shell pwd)" RUSTFLAGS="-Z emit-stack-sizes" cargo build --out-dir=build -Z build-std=core,alloc -Z features=host_dep -Z unstable-options $(FEATURES)

.PHONY: domains
domains:
	cargo rbuild $(CARGO_FLAGS) $(DOMAIN_FEATURES)

.PHONY: check
check:
	cargo rcheck $(CARGO_FLAGS) $(DOMAIN_FEATURES)

.PHONY: clean
clean:
	cargo clean
	rm -rf build

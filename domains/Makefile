.PHONY: domains
domains:
	-find target -type f -name "libmembdev*" -delete
	cargo rbuild $(CARGO_FLAGS) $(DOMAIN_FEATURES)

.PHONY: check
check:
	cargo rcheck $(CARGO_FLAGS) $(DOMAIN_FEATURES)

.PHONY: clean
clean:
	cargo clean
	rm -rf build

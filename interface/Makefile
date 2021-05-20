ROOT := ../
OUTPUT_DIR = generated
LIB_RS = $(OUTPUT_DIR)/src/lib.rs
MANIFEST_PATH = $(OUTPUT_DIR)/Cargo.toml
NGC = RUST_LOG=INFO RUST_BACKTRACE=1 cargo run --manifest-path $(ROOT)/tools/redIDL/codegen/ngc/Cargo.toml --release 
DOMAIN_CREATE_OUTPUT_PATH = ../kernel/src/generated_domain_create.rs

.PHONY: all
all: $(LIB_RS) $(MANIFEST_PATH)

# Copy Cargo.toml and fix dependency path
$(MANIFEST_PATH): Cargo.toml missing_dependencies.toml
	mkdir -p $(OUTPUT_DIR)
	sed -E 's/path\s*=\s*"/path = "..\//' $(word 1,$^)  > $@
	cat $(word 2,$^) >> $@

# Merge all files and inject macro for expansion
.PHONY: $(OUTPUT_DIR)/merged.rs
$(OUTPUT_DIR)/merged.rs:
	mkdir -p $(OUTPUT_DIR)
	cargo expand > $@

# Inject use statements
$(LIB_RS): $(OUTPUT_DIR)/merged.rs
	mkdir -p $(OUTPUT_DIR)/src
	$(NGC) $< $@ --domain_create_output=$(DOMAIN_CREATE_OUTPUT_PATH)

.PHONY: clean
clean:
	-rm -rf $(OUTPUT_DIR)
	-rm -f $(DOMAIN_CREATE_OUTPUT_PATH)
	




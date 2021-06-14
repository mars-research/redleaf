ROOT := ../
OUTPUT_DIR = generated
LIB_RS = $(OUTPUT_DIR)/src/lib.rs
MANIFEST_PATH = $(OUTPUT_DIR)/Cargo.toml
NGC = RUST_LOG=INFO RUST_BACKTRACE=1 cargo run --manifest-path $(ROOT)/tools/redIDL/codegen/ngc/Cargo.toml --release 
DOMAIN_CREATE_OUTPUT_PATH = ../kernel/src/generated_domain_create.rs

.PHONY: all
all: $(LIB_RS) $(MANIFEST_PATH)

# Copy Cargo.toml, and fix dependency path, remove `interface_attribute_placeholder`, and add
# missing dependencies.
$(MANIFEST_PATH): Cargo.toml missing_dependencies.toml
	mkdir -p $(OUTPUT_DIR)
	sed -E 's/path\s*=\s*"/path = "..\//' $(word 1,$^)  > $@
	cat $(word 2,$^) >> $@

# Add placeholder dependencies and run cargo check.
# Then restore the Cargo.toml.
# TODO(issue #49): move the operation to a temp folder to prevent contaminating the source folder.
.PHONY: compiler_check
compiler_check:
	cp Cargo.toml Cargo.toml.backup
	cat Cargo.toml.backup placeholder_dependencies.toml > Cargo.toml
	cargo check
	mv Cargo.toml.backup Cargo.toml

# Merge all files and inject macro for expansion.
# Since the placeholder removes the interface attributes from the traits, we need to remove
# them from the file.
.PHONY: $(OUTPUT_DIR)/merged.rs
$(OUTPUT_DIR)/merged.rs: compiler_check
	mkdir -p $(OUTPUT_DIR)
	cp src/lib.rs src/lib.rs.backup
	tac src/lib.rs.backup | sed '/extern crate interface_attribute_placeholder;/I,+1 d' | tac > src/lib.rs
	cargo expand > $@
	mv src/lib.rs.backup src/lib.rs

# Inject use statements
$(LIB_RS): $(OUTPUT_DIR)/merged.rs
	mkdir -p $(OUTPUT_DIR)/src
	$(NGC) $< $@ --domain_create_output=$(DOMAIN_CREATE_OUTPUT_PATH)

.PHONY: clean
clean:
	cargo clean
	-rm -rf $(OUTPUT_DIR)
	-rm -f $(DOMAIN_CREATE_OUTPUT_PATH)
	




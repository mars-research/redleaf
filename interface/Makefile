TEMP_DIR = target/codegen
LIB_RS = $(TEMP_DIR)/src/lib.rs
ROOT := ../

REMOVE_PRELUDE = awk -i inplace '!/\#!\[feature\(prelude_import)]|\#\[prelude_import]|use core::prelude::v1::\*;/' $(TEMP_DIR)/src/lib.rs

.PHONY: all
all: $(TEMP_DIR)/expanded.rs

# Copy Cargo.toml and fix dependency path
$(TEMP_DIR)/Cargo.toml: Cargo.toml missing_dependencies.toml
	sed -E 's/path\s*=\s*"/path = "..\/..\//' $< > $@
	cat $(word 2,$^) >> $@

# Merge all files and inject macro for expansion
.PHONY: $(TEMP_DIR)/merged.rs
$(TEMP_DIR)/merged.rs: 
	mkdir -p $(TEMP_DIR)/src
	cargo expand > $@

# Inject use statements
$(TEMP_DIR)/fix_dependency.rs: $(TEMP_DIR)/merged.rs
	cargo run --manifest-path $(ROOT)/tools/redIDL/codegen/codegen-proxy/Cargo.toml $< $@

$(TEMP_DIR)/expanded.rs: $(TEMP_DIR)/fix_dependency.rs $(TEMP_DIR)/Cargo.toml
	cp $< $(LIB_RS)
	-rustfmt $(LIB_RS)
	cargo expand --manifest-path $(word 2,$^) > $@



.PHONY: clean
clean:
	-rm -rf $(TEMP_DIR)
	




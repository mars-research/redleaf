TEMP_DIR = target/codegen
LIB_RS = $(TEMP_DIR)/src/lib.rs
MANIFEST_PATH = $(TEMP_DIR)/Cargo.toml
CLEANSE = cargo run --manifest-path $(ROOT)/tools/redIDL/codegen/cleanser/Cargo.toml
ROOT := ../

REMOVE_PRELUDE = awk -i inplace '!/\#!\[feature\(prelude_import)]|\#\[prelude_import]|use core::prelude::v1::\*;/' $(TEMP_DIR)/src/lib.rs

.PHONY: all
all: $(TEMP_DIR)/expanded.rs

# Copy Cargo.toml and fix dependency path
$(MANIFEST_PATH): Cargo.toml missing_dependencies.toml
	sed -E 's/path\s*=\s*"/path = "..\/..\//' $< > $@
	cat $(word 2,$^) >> $@

# .PHONY: replace_definitions
# replace_definitions:
# 	rm -rf $(TEMP_DIR)/src
# 	mkdir -p $(TEMP_DIR)/src
# 	$(CLEANSE) './src/*' $(TEMP_DIR)/src --directory-mode --replace-definitions
# 	sed -i '1i #![feature(extended_key_value_attributes)]' $(LIB_RS)


# Merge all files and inject macro for expansion
.PHONY: $(TEMP_DIR)/merged.rs
$(TEMP_DIR)/merged.rs: $(MANIFEST_PATH)
	echo "#![feature(extended_key_value_attributes)]\n" > $@
	cargo expand >> $@

# Inject use statements
$(TEMP_DIR)/fix_dependency.rs: $(TEMP_DIR)/merged.rs
	$(CLEANSE) $< $@ --remove-prelude --inject-dependency --fix-use --replace-definitions

$(TEMP_DIR)/resolve_path_for_definition_replacement.rs: $(TEMP_DIR)/fix_dependency.rs
	cp $< $(LIB_RS)
	cargo expand --manifest-path $(MANIFEST_PATH) > $@

$(TEMP_DIR)/definition_replaced.rs: $(TEMP_DIR)/resolve_path_for_definition_replacement.rs
	cp $< $(LIB_RS)
	sed -i -e 's/redidl_generate_import_placeholder_/redidl_generate_import/g' $(LIB_RS)
	cargo expand --manifest-path $(MANIFEST_PATH) > $@


$(TEMP_DIR)/pre_module_resolution.rs: $(TEMP_DIR)/fix_dependency.rs 
	cp $< $(LIB_RS)
	cargo expand --manifest-path $(MANIFEST_PATH) > $@

$(TEMP_DIR)/module_resolved.rs: $(TEMP_DIR)/definition_replaced.rs
	$(CLEANSE) $^ $(LIB_RS) --remove-prelude
	cargo expand --manifest-path $(MANIFEST_PATH) > $@

$(TEMP_DIR)/expanded.rs: $(TEMP_DIR)/module_resolved.rs
	$(CLEANSE) $^ $(LIB_RS) --remove-prelude
	sed -i -e 's/redidl_codegen_generate_proxy_placeholder_/redidl_generate_proxy/g' $(LIB_RS)
	cargo expand --manifest-path $(MANIFEST_PATH) > $@


.PHONY: clean
clean:
	-rm -rf $(TEMP_DIR)
	




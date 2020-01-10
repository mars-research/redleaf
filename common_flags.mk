DEBUG = true
TARGET_SUB_DIR = debug

CARGO_FLAGS ?=
ifeq ($(DEBUG),false)
	CARGO_FLAGS += --release
	TARGET_SUB_DIR = release
endif
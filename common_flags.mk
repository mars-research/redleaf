DEBUG = true
TARGET_SUB_DIR = debug

CARGO_FLAGS ?=
CARGO_COMMAND = xbuild

ifeq ($(DEBUG),false)
	CARGO_FLAGS += --release
	TARGET_SUB_DIR = release
endif
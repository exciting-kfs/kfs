
# === Top level settings ===

RELEASE_MODE := y
DEBUG_WITH_VSCODE := y
TEST_CASE := all
HDD_FAST_BUILD := n
LOG_LEVEL := info # ALL = debug > info > warn > error

# === Target names ===

LIB_KERNEL_NAME := libkernel.a
KERNEL_BIN_NAME := kernel
KERNEL_MODULE_NAMES := kbd timestamp
KERNEL_DEBUG_SYMBOL_NAME := $(KERNEL_BIN_NAME).sym

export USER_BIN_NAMES := init shell test_pipe test_sig test_setXid test_sig_stop_cont test_file test_socket getty test test_argv test_mmap test_sleep

RESCUE_IMG_NAME := rescue.iso
HDD_IMG_NAME := disk.qcow2


# === Root directory ===

ifeq ($(RELEASE_MODE),y)
TARGET_BUILD_MODE := release
else
TARGET_BUILD_MODE := debug
endif

TARGET_ROOT := target/i686-unknown-none-elf/$(TARGET_BUILD_MODE)
USER_SRC_ROOT := userspace
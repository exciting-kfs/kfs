# === OS ===

OS := $(shell uname -s)
ifeq ($(OS), Linux)
UTIL := linux-gnu
LDFLAG = -n --script=$(LINKER_SCRIPT) --gc-sections
else
UTIL := elf
LDFLAG = -n --no-warn-rwx-segments --no-warn-execstack --script=$(LINKER_SCRIPT) --gc-sections
endif

# === User settings / toolchain ===

RELEASE_MODE := n
DEBUG_WITH_VSCODE := y
TEST_CASE := all
# LOG_LEVEL := debug # ALL = debug > info > warn > error

I386_GRUB2_PREFIX := $(I386_GRUB2_PREFIX)

OBJCOPY := i686-$(UTIL)-objcopy

OBJDUMP := i686-$(UTIL)-objdump
OBJDUMP_FLAG := --demangle                                  \
                --disassembler-options=intel,intel-mnemonic \

LD := i686-$(UTIL)-ld

ADDR2LINE := i686-$(UTIL)-addr2line

PAGER := vim -

# === compiler flag ===

# RUSTC_FLAG += --cfg log_level='"$(LOG_LEVEL)"' 

# === toolchain (inferred from above) ===

GRUB2_MKRESCUE=$(I386_GRUB2_PREFIX)/bin/grub-mkrescue
GRUB2_I386_LIB=$(I386_GRUB2_PREFIX)/lib/grub/i386-pc

# === Targets ===

ifeq ($(RELESE_MODE),y)
TARGET_ROOT := target/i686-unknown-none-elf/release
CARGO_FLAG :=  --release
else
TARGET_ROOT := target/i686-unknown-none-elf/debug
endif

LIB_KERNEL_NAME := libkernel.a
LIB_KERNEL_SRC_ROOT := src

LIB_KERNEL := $(TARGET_ROOT)/$(LIB_KERNEL_NAME)
LIB_KERNEL_SRC := $(shell find $(LIB_KERNEL_SRC_ROOT) -type f -and \( -name '*.[sS]' -or -name '*.rs' \))
CARGO_CONFIG := Cargo.toml .cargo/config.toml
BUILD_SCRIPT := build.rs

KERNEL_BIN_NAME := kernel
KERNEL_BIN := $(TARGET_ROOT)/$(KERNEL_BIN_NAME)

KERNEL_ELF_NAME := $(KERNEL_BIN_NAME).elf
KERNEL_ELF := $(TARGET_ROOT)/$(KERNEL_ELF_NAME)

KERNEL_DEBUG_SYMBOL_NAME := $(KERNEL_BIN_NAME).sym
KERNEL_DEBUG_SYMBOL := $(TARGET_ROOT)/$(KERNEL_DEBUG_SYMBOL_NAME)

RESCUE_TARGET_ROOT := $(TARGET_ROOT)/iso
RESUCE_SRC_ROOT := iso

RESCUE_IMG_NAME := rescue.iso
RESCUE_IMG := $(TARGET_ROOT)/$(RESCUE_IMG_NAME)

LINKER_SCRIPT := linker-script/kernel.ld

DOC := $(shell dirname $(TARGET_ROOT))/doc/kernel/index.html

# === user space targets

USERSPACE_SRC_ROOT := userspace

# === Phony recipes ===

.PHONY : all
all : rescue

.PHONY : build
build : $(KERNEL_BIN)

.PHONY : rescue
rescue : $(RESCUE_IMG)

.PHONY : clean
clean :
	@echo '[-] cleanup...'
	@cargo clean -v
	@rm -f .sw*
	@$(MAKE) -s -C $(USERSPACE_SRC_ROOT) clean

.PHONY : re
re : clean
	@$(MAKE) all

.PHONY : run
run : rescue
	@scripts/qemu.sh $(RESCUE_IMG) stdio -monitor pty

.PHONY : debug debug-display
ifeq ($(DEBUG_WITH_VSCODE),y)
debug : $(RESCUE_IMG) $(KERNEL_DEBUG_SYMBOL)
	@scripts/vsc-debug.py $(KERNEL_DEBUG_SYMBOL) $(KERNEL_BIN) &
	@scripts/qemu.sh $(RESCUE_IMG) stdio -s -S -monitor pty -display none 

debug-display : $(RESCUE_IMG) $(KERNEL_DEBUG_SYMBOL)
	@scripts/vsc-debug.py $(KERNEL_DEBUG_SYMBOL) $(KERNEL_BIN) &
	@scripts/qemu.sh $(RESCUE_IMG) stdio -s -S -monitor pty

else
debug : $(RESCUE_IMG) $(KERNEL_DEBUG_SYMBOL)
	@scripts/qemu.sh $(RESCUE_IMG) stdio -s -S -monitor pty & rust-lldb   \
		--one-line "target create --symfile $(KERNEL_DEBUG_SYMBOL) $(KERNEL_BIN)"   \
		--one-line "gdb-remote localhost:1234"                                      \
		--source scripts/debug.lldb
endif

.PHONY : doc
doc :
	@cargo doc $(CARGO_FLAG)

.PHONY : doc-open
doc-open : doc
	@open $(DOC)

# Prepend PIPE operator only if PAGER is set.
ifdef PAGER
PAGER := | $(PAGER)
endif

.PHONY : dump-header
dump-header :
	@$(OBJDUMP) $(OBJDUMP_FLAG) --all-headers $(KERNEL_BIN) $(PAGER)

.PHONY : dump-text
dump-text :
	@$(OBJDUMP) $(OBJDUMP_FLAG) --disassemble $(KERNEL_BIN) $(PAGER)

.PHONY : check-stack
check-stack :
	@$(MAKE) dump-text | python3 scripts/checkstack.py

.PHONY : size
size :
	@ls -lh $(KERNEL_BIN)

.PHONY : lookup-addr
lookup-addr :
ifndef ADDR
	@echo "Usage: make ADDR=\`address\` lookup-addr"
	@exit 2
else
	@$(ADDR2LINE) -e $(KERNEL_DEBUG_SYMBOL) $(ADDR) 
endif

.PHONY : test
test : RUSTC_FLAG += --cfg ktest
test : RUSTC_FLAG += --cfg ktest='"$(TEST_CASE)"'
test : rescue
	@scripts/qemu.sh $(RESCUE_IMG) stdio -display none

# === Main recipes ===

.PHONY : $(LIB_KERNEL)
$(LIB_KERNEL) : userspace
	@cargo rustc $(CARGO_FLAG) -- $(RUSTC_FLAG)

# TODO: better dependency tracking.
#
# $(LIB_KERNEL) : $(LIB_KERNEL_SRC) $(BUILD_SCRIPT) $(CARGO_CONFIG)
# 	@cargo build

$(KERNEL_ELF) : $(LIB_KERNEL) $(LINKER_SCRIPT)
	@echo "[-] linking kernel image..."
	@$(LD) $(LDFLAG)		\
		--whole-archive		\
		$(LIB_KERNEL)		\
		-o $@

$(KERNEL_BIN) : $(KERNEL_ELF)
	@echo "[-] stripping debug-symbols..."
	@$(OBJCOPY) --strip-debug $< $(KERNEL_BIN)

$(KERNEL_DEBUG_SYMBOL) : $(KERNEL_ELF)
	@echo "[-] extracting debug-symbols..."
	@$(OBJCOPY) --only-keep-debug $< $(KERNEL_DEBUG_SYMBOL)

$(RESCUE_IMG) : $(KERNEL_BIN) $(shell find $(RESUCE_SRC_ROOT) -type f) $(KERNEL_DEBUG_SYMBOL)
	@echo "[-] creating rescue image..."
	@mkdir -p $(TARGET_ROOT)/boot
	@cp -r $(RESUCE_SRC_ROOT) $(TARGET_ROOT)
	@cp $(KERNEL_BIN) $(RESCUE_TARGET_ROOT)/boot
	@$(GRUB2_MKRESCUE) -d $(GRUB2_I386_LIB) $(RESCUE_TARGET_ROOT) -o $@ 2>/dev/null >/dev/null

.PHONY : userspace
userspace :
	@echo "[-] build userspace binaries"
	@$(MAKE) -s -C $(USERSPACE_SRC_ROOT)

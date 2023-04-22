# === User settings / toolchain ===

RELEASE_MODE := n
DEBUG_WITH_VSCODE := y

I386_GRUB2_PREFIX := $(I386_GRUB2_PREFIX)

OBJCOPY := i686-elf-objcopy

OBJDUMP := i686-elf-objdump
OBJDUMP_FLAG := --demangle                                  \
                --disassembler-options=intel,intel-mnemonic \

LD := i686-elf-ld

PAGER := vim -

# === toolchain (inferred from above) ===

GRUB2_MKRESCUE=$(I386_GRUB2_PREFIX)/bin/grub-mkrescue
GRUB2_I386_LIB=$(I386_GRUB2_PREFIX)/lib/grub/i386-pc

# === Targets ===

LDFLAG = -n --no-warn-rwx-segments --no-warn-execstack --script=$(LINKER_SCRIPT) --gc-sections

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

DOC := $(TARGET_ROOT)/doc/kernel/index.html

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
	@cargo clean
	@rm -f .sw*

.PHONY : re
re : clean
	@$(MAKE) all

.PHONY : run
run : rescue
	@scripts/qemu.sh $(RESCUE_IMG) stdio

.PHONY : debug
ifeq ($(DEBUG_WITH_VSCODE),y)
debug : $(RESCUE_IMG) $(KERNEL_DEBUG_SYMBOL)
	@scripts/vsc-debug.py $(KERNEL_DEBUG_SYMBOL) $(KERNEL_BIN) &
	@scripts/qemu.sh $(RESCUE_IMG) stdio -s -S
else
debug : $(RESCUE_IMG) $(KERNEL_DEBUG_SYMBOL)
	@scripts/qemu.sh $(RESCUE_IMG) stdio -s -S & rust-lldb   \
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
dump-header : $(KERNEL_BIN)
	@$(OBJDUMP) $(OBJDUMP_FLAG) --all-headers $(KERNEL_BIN) $(PAGER)

.PHONY : dump-text
dump-text : $(KERNEL_BIN)
	@$(OBJDUMP) $(OBJDUMP_FLAG) --disassemble $(KERNEL_BIN) $(PAGER)

.PHONY : size
size : $(KERNEL_BIN)
	@ls -lh $<

.PHONY : test
test : RUSTC_FLAG += --cfg ktest
test : rescue
	@scripts/qemu.sh $(RESCUE_IMG) stdio -display none

# === Main recipes ===

.PHONY : $(LIB_KERNEL)
$(LIB_KERNEL) :
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

$(RESCUE_IMG) : $(KERNEL_BIN) $(shell find $(RESUCE_SRC_ROOT) -type f)
	@echo "[-] creating rescue image..."
	@mkdir -p $(TARGET_ROOT)/boot
	@cp -r $(RESUCE_SRC_ROOT) $(TARGET_ROOT)
	@cp $(KERNEL_BIN) $(RESCUE_TARGET_ROOT)/boot
	@$(GRUB2_MKRESCUE) -d $(GRUB2_I386_LIB) $(RESCUE_TARGET_ROOT) -o $@ 2>/dev/null >/dev/null

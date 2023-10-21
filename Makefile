# === OS ===

OS := $(shell uname -s)
ifeq ($(OS), Linux)
PREFIX := i686-linux-gnu-
LDFLAG = -n 
else
PREFIX := i686-elf-
LDFLAG = -n --no-warn-rwx-segments --no-warn-execstack
endif

# === User settings / toolchain ===

RELEASE_MODE := n
DEBUG_WITH_VSCODE := y
TEST_CASE := all
FAST_HDD_BUILD := y

# LOG_LEVEL := debug # ALL = debug > info > warn > error

I386_GRUB2_PREFIX := $(I386_GRUB2_PREFIX)

OBJCOPY := $(PREFIX)objcopy

OBJDUMP := $(PREFIX)objdump
OBJDUMP_FLAG := --demangle                                  \
                --disassembler-options=intel,intel-mnemonic \

LD := $(PREFIX)ld

ADDR2LINE := $(PREFIX)addr2line

PAGER := vim -

# === compiler flag ===

# RUSTC_FLAG += --cfg log_level='"$(LOG_LEVEL)"' 

# === toolchain (inferred from above) ===

GRUB2_MKRESCUE=$(I386_GRUB2_PREFIX)/bin/grub-mkrescue
GRUB2_I386_LIB=$(I386_GRUB2_PREFIX)/lib/grub/i386-pc

# === Targets ===

ifeq ($(RELEASE_MODE),y)
TARGET_ROOT := target/i686-unknown-none-elf/release
CARGO_FLAG :=  --release
else
TARGET_ROOT := target/i686-unknown-none-elf/debug
endif

KERNEL_MODULE_NAMES := kbd

KERNEL_MODULES := $(addprefix $(TARGET_ROOT)/,$(KERNEL_MODULE_NAMES))
KERNEL_MODULES := $(addsuffix .ko,$(KERNEL_MODULES))

KERNEL_MODULE_LIBS := $(addprefix lib,$(KERNEL_MODULE_NAMES))
KERNEL_MODULE_LIBS := $(addsuffix .a,$(KERNEL_MODULE_LIBS))
KERNEL_MODULE_LIBS := $(addprefix $(TARGET_ROOT)/,$(KERNEL_MODULE_LIBS))

LIB_KERNEL_NAME := libkernel.a

LIB_KERNEL := $(TARGET_ROOT)/$(LIB_KERNEL_NAME)

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

HDD_IMG_NAME := disk.qcow2
HDD_IMG := $(TARGET_ROOT)/$(HDD_IMG_NAME)

LINKER_SCRIPT := linker-script/kernel.ld

DOC := $(shell dirname $(TARGET_ROOT))/doc/kernel/index.html

CARGO_TARGETS := $(addprefix cargo-buildlib-,kfs $(KERNEL_MODULE_NAMES))

# === user space targets

USERSPACE_SRC_ROOT := userspace

# === Phony recipes ===

.PHONY : all
all : rescue hdd modules
	@mkdir -p log

.PHONY : build
build : $(KERNEL_BIN)

.PHONY : rescue
rescue : $(RESCUE_IMG)

.PHONY : userspace
userspace :
	@echo MAKE $@
	@$(MAKE) EXTRA_CFLAGS=$(CFLAGS) -s -C $(USERSPACE_SRC_ROOT)

.PHONY : ci
ci : export CFLAGS := -Werror
ci : export RUSTC_FLAG += -D warnings
ci : test

.PHONY: hdd
hdd: $(HDD_IMG)

.PHONY : clean
clean :
	@echo 'CARGO clean'
	@cargo clean -v
	@echo 'RM .sw* log/'
	@rm -f .sw*
	@rm -rf log/
	@echo 'MAKE clean'
	@$(MAKE) -s -C $(USERSPACE_SRC_ROOT) clean

.PHONY : re
re : clean
	@$(MAKE) all

.PHONY : run
run : all
	@scripts/qemu.sh $(RESCUE_IMG) $(HDD_IMG) stdio -monitor pty

.PHONY : debug debug-display
ifeq ($(DEBUG_WITH_VSCODE),y)
debug : all $(KERNEL_DEBUG_SYMBOL)
	@scripts/vsc-debug.py $(KERNEL_DEBUG_SYMBOL) $(KERNEL_BIN) &
	@scripts/qemu.sh $(RESCUE_IMG) $(HDD_IMG) stdio -s -S -monitor pty -display none 

debug-display : all $(KERNEL_DEBUG_SYMBOL)
	@scripts/vsc-debug.py $(KERNEL_DEBUG_SYMBOL) $(KERNEL_BIN) &
	@scripts/qemu.sh $(RESCUE_IMG) $(HDD_IMG) stdio -s -S -monitor pty

else
debug : all $(KERNEL_DEBUG_SYMBOL)
	@scripts/qemu.sh $(RESCUE_IMG) $(HDD_IMG) stdio -s -S -monitor pty & rust-lldb   \
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
	@$(MAKE) PAGER='' dump-text | python3 scripts/checkstack.py

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
test : export RUSTC_FLAG += --cfg ktest
test : export RUSTC_FLAG += --cfg ktest='"$(TEST_CASE)"'
test : all
	@scripts/qemu.sh $(RESCUE_IMG) $(HDD_IMG) stdio -display none

# === Main recipes ===

.PHONY : modules
modules : $(KERNEL_MODULES)

.PHONY : $(CARGO_TARGETS)
$(CARGO_TARGETS) :
	@echo CARGO lib$(subst cargo-buildlib-,,$@).a
	@cargo rustc -p $(subst cargo-buildlib-,,$@) $(CARGO_FLAG) -- $(RUSTC_FLAG)

$(KERNEL_MODULE_LIBS) : $(TARGET_ROOT)/lib%.a : cargo-buildlib-%

$(KERNEL_MODULES) : $(TARGET_ROOT)/%.ko : $(TARGET_ROOT)/lib%.a $(LIB_KERNEL)
	@echo LD $(patsubst %.ko,lib%.a,$(notdir $@))
	@$(LD) $(LDFLAG)		\
		--whole-archive		\
		-R $(KERNEL_BIN)	\
		-r					\
		-o $@				\
		$<
	@echo OBJCOPY $(notdir $@)
	@$(OBJCOPY) --strip-debug $@

$(LIB_KERNEL) : userspace
	@$(MAKE) cargo-buildlib-kfs

$(KERNEL_ELF) : $(LIB_KERNEL) $(LINKER_SCRIPT)
	@echo LD $(notdir $@)
	@$(LD) $(LDFLAG)		\
		--whole-archive		\
		-T $(LINKER_SCRIPT) \
		-o $@				\
		$(LIB_KERNEL)

$(KERNEL_BIN) : $(KERNEL_ELF)
	@echo OBJCOPY $(notdir $@)
	@$(OBJCOPY) --strip-debug $< $(KERNEL_BIN)

$(KERNEL_DEBUG_SYMBOL) : $(KERNEL_ELF)
	@echo OBJCOPY $(notdir $@)
	@$(OBJCOPY) --only-keep-debug $< $(KERNEL_DEBUG_SYMBOL)

$(RESCUE_IMG) : $(KERNEL_BIN) $(shell find $(RESUCE_SRC_ROOT) -type f) $(KERNEL_DEBUG_SYMBOL)
	@echo MKRESCUE $(notdir $@)
	@mkdir -p $(TARGET_ROOT)/boot
	@cp -r $(RESUCE_SRC_ROOT) $(TARGET_ROOT)
	@cp $(KERNEL_BIN) $(RESCUE_TARGET_ROOT)/boot
	@$(GRUB2_MKRESCUE) -d $(GRUB2_I386_LIB) $(RESCUE_TARGET_ROOT) -o $@ 2>/dev/null >/dev/null

$(TARGET_ROOT)/sysroot : $(KERNEL_MODULES)
	@echo MAKE sysroot
	@rm -rf $(TARGET_ROOT)/sysroot
	@mkdir -p $(TARGET_ROOT)/sysroot
	@cp $(KERNEL_MODULES) $(TARGET_ROOT)/sysroot

$(HDD_IMG) : $(TARGET_ROOT)/sysroot scripts/hdd/make-hdd.sh scripts/hdd/make-hdd-linux.sh
	@echo MAKE $(notdir $@)
ifeq ($(FAST_HDD_BUILD),y)
	@scripts/hdd/make-hdd-linux.sh $@ $(TARGET_ROOT)/sysroot
else
	@scripts/hdd/make-hdd.sh $@ $(TARGET_ROOT)/sysroot
endif

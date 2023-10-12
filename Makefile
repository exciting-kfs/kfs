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

ifeq ($(RELESE_MODE),y)
TARGET_ROOT := target/i686-unknown-none-elf/release
CARGO_FLAG :=  --release
else
TARGET_ROOT := target/i686-unknown-none-elf/debug
endif

KERNEL_MODULE_NAMES := hello

KERNEL_MODULES := $(addprefix $(TARGET_ROOT)/,$(KERNEL_MODULE_NAMES))
KERNEL_MODULES := $(addsuffix .ko,$(KERNEL_MODULES))

KERNEL_MODULE_LIBS := $(addprefix lib,$(KERNEL_MODULE_NAMES))
KERNEL_MODULE_LIBS := $(addsuffix .a,$(KERNEL_MODULE_LIBS))
KERNEL_MODULE_LIBS := $(addprefix $(TARGET_ROOT)/,$(KERNEL_MODULE_LIBS))

LIB_KERNEL_NAME := libkernel.a
LIB_KERNEL_SRC_ROOT := src

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

HDD_IMG_NAME := disk.iso
HDD_IMG := $(TARGET_ROOT)/$(HDD_IMG_NAME)

LINKER_SCRIPT := linker-script/kernel.ld

DOC := $(shell dirname $(TARGET_ROOT))/doc/kernel/index.html

# === user space targets

USERSPACE_SRC_ROOT := userspace

# === Phony recipes ===

.PHONY : all
all : rescue hdd
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
ci : CFLAGS := -Werror
ci : RUSTC_FLAG += -D warnings
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
run : all hello.txt
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
test : RUSTC_FLAG += --cfg ktest
test : RUSTC_FLAG += --cfg ktest='"$(TEST_CASE)"'
test : all 
	@scripts/qemu.sh $(RESCUE_IMG) $(HDD_IMG) stdio -display none

# === Main recipes ===

modules : $(KERNEL_MODULES)

.PHONY : $(KERNEL_MODULE_LIBS)
$(KERNEL_MODULE_LIBS) :
	@echo CARGO $(notdir $@)
	@cargo rustc -p $(patsubst lib%.a,%,$(notdir $@)) $(CARGO_FLAG) -- $(RUSTC_FLAG)

$(TARGET_ROOT)/%.ko : $(TARGET_ROOT)/lib%.a $(KERNEL_BIN)
	@echo LD $(notdir $@)
	@$(LD) $(LDFLAG)		\
		--whole-archive		\
		-R $(KERNEL_BIN)	\
		-r					\
		-o $@				\
		$<
	@echo OBJCOPY $(notdir $@)
	@$(OBJCOPY) --strip-debug $@

.PHONY : $(LIB_KERNEL)
$(LIB_KERNEL) : userspace
	@echo CARGO $(notdir $@)
	@cargo rustc $(CARGO_FLAG) -- $(RUSTC_FLAG)

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

$(HDD_IMG) : $(KERNEL_MODULES) scripts/hdd/make-hdd.sh
	@echo BUILD $(notdir $@)
	@scripts/hdd/make-hdd.sh $@

hello.txt: 
	@dd if=/dev/zero of=hello.txt bs=1024 count=1024

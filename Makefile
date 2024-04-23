-include build/config.mk

# === OS ===

OS := $(shell uname -s)
ifeq ($(OS), Linux)
PREFIX := i686-linux-gnu-
LDFLAG = -n 
else
PREFIX := i686-elf-
LDFLAG = -n --no-warn-rwx-segments --no-warn-execstack
endif

# === toolchain ===

I386_GRUB2_PREFIX := $(I386_GRUB2_PREFIX)

OBJCOPY := $(PREFIX)objcopy

OBJDUMP := $(PREFIX)objdump
OBJDUMP_FLAG := --demangle                                  \
                --disassembler-options=intel,intel-mnemonic \

LD := $(PREFIX)ld

ADDR2LINE := $(PREFIX)addr2line

PAGER := vim -

# === Targets ===
LIB_KERNEL := $(TARGET_ROOT)/$(LIB_KERNEL_NAME)
KERNEL_BIN := $(TARGET_ROOT)/$(KERNEL_BIN_NAME)
KERNEL_DEBUG_SYMBOL := $(TARGET_ROOT)/$(KERNEL_DEBUG_SYMBOL_NAME)
KERNEL_MODULES := $(shell for x in $(KERNEL_MODULE_NAMES); do printf "$(TARGET_ROOT)/%s.ko " $$x; done)

RESCUE_IMG := $(TARGET_ROOT)/$(RESCUE_IMG_NAME)
HDD_IMG := $(TARGET_ROOT)/$(HDD_IMG_NAME)

DOC := $(shell dirname $(TARGET_ROOT))/doc/kernel/index.html

USER_BINS := $(addprefix $(USER_SRC_ROOT)/build/, $(USER_BIN_NAMES))

# === Project management recipes ===
.PHONY : all clean re run
all : rescue hdd modules userspace

clean :
	@echo 'CARGO clean'
	@cargo clean -v
	@echo 'RM .sw* log/'
	@rm -f .sw*
	@rm -rf log/
	@echo 'MAKE clean'
	@$(MAKE) -s -C $(USER_SRC_ROOT) clean

re : clean
	@$(MAKE) all

run : all
	@scripts/qemu.sh $(RESCUE_IMG) $(HDD_IMG) stdio -monitor pty

# === Target recipes ===
.PHONY : kernel rescue modules userspace hdd hdd-force doc doc-open
kernel : $(KERNEL_BIN)
-include build/kernel.mk

rescue : $(RESCUE_IMG)
-include build/rescue.mk

modules : $(KERNEL_MODULES)
-include build/module.mk

userspace : $(USER_BINS)
-include build/userbin.mk

## Cargo
-include build/cargo.mk

hdd: $(HDD_IMG)
-include build/hdd.mk

hdd-force :
	rm $(HDD_IMG)
	$(MAKE) hdd

doc :
	@cargo doc $(CARGO_FLAG)

doc-open : doc
	@open $(DOC)

# === Debuging recipes ===

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

ifdef PAGER # Prepend PIPE operator only if PAGER is set.
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

# === Testing recipes ===
.PHONY : test
test : export RUSTC_FLAG += --cfg ktest
test : export RUSTC_FLAG += --cfg ktest='"$(TEST_CASE)"'
test : rescue 
	@scripts/qemu.sh $(RESCUE_IMG) - stdio -display none

.PHONY : ci
ci : export CFLAGS := -Werror
ci : export RUSTC_FLAG += -D warnings
ci : test

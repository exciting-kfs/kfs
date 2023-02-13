KERNEL_BIN := target/i686-unknown-none-elf/debug/kernel
OBJDUMP_OPTS := --demangle --x86-asm-syntax=intel --print-imm-hex
OBJDUMP_PAGER := | vim -

all : rescue

build :
	cargo build

doc :
	cargo doc

# it works on macos, but not tested on another os
doc-open : doc
	open target/i686-unknown-none-elf/doc/kernel/index.html

run :
	scripts/qemu.sh

rescue :
	scripts/rescue.sh

clean :
	cargo clean

dump-all:
	objdump -D $(OBJDUMP_OPTS) $(KERNEL_BIN) $(OBJDUMP_PAGER)

dump-text:
	objdump -d $(OBJDUMP_OPTS) $(KERNEL_BIN) $(OBJDUMP_PAGER)

dump-header:
	objdump -x $(KERNEL_BIN) $(OBJDUMP_PAGER)

debug :
	scripts/debug.sh &
	rust-lldb $(KERNEL_BIN)                    \
		--one-line 'gdb-remote localhost:1234' \
		--one-line 'b kernel_init'             \
		--one-line 'c'
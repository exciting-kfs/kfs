KERNEL_BIN := target/iso/boot/kernel
OBJDUMP_OPTS := -x86-asm-syntax=intel --print-imm-hex

all : build rescue

build :
	cargo build

run :
	scripts/qemu.sh

rescue :
	scripts/rescue.sh
	
clean :
	cargo clean

dump-all:
	objdump -D $(OBJDUMP_OPTS) $(KERNEL_BIN) | less

dump-text:
	objdump -d $(OBJDUMP_OPTS) $(KERNEL_BIN) | less

dump-header:
	objdump -x $(KERNEL_BIN) | less

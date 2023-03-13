ENTRY(kernel_init)

EXTERN(multiboot2_header_start)

SECTIONS {

	. = 1M;

	.boot : ALIGN(64) {
		KEEP(*(.multiboot2_header))
	}

	.gdt : ALIGN(8) {
		KEEP(*(.gdt))
	}

	.text : ALIGN(4096) {
		*(.text .text.*)
	}

	.data : ALIGN(4096) {
		*(.data .data.*)
	}

	.rodata : ALIGN(4096) {
		*(.rodata .rodata.*)
	}

	.bss : ALIGN(4096) {
		*(.bss .bss.*)
	}

	.kernel_stack : ALIGN(4096) {
		*(.kernel_stack)
	}
}

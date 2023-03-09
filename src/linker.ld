ENTRY(kernel_init)

EXTERN(multiboot2_header_start)

SECTIONS {
	. = 1M;
	.boot : ALIGN(64) {
		KEEP(*(.multiboot2_header))
	}

	. = 2M;
	.text : {
		*(.text .text.*)
	}

}

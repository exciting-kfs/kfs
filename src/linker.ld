ENTRY(kernel_init)

EXTERN(multiboot2_header_start)

SECTIONS {
	. = 1M;

	.boot : ALIGN(64) {
		KEEP(*(.multiboot2_header))
	}

    .text : {
		*(.text .text.*)
	}

 	.stack : ALIGN(16) {
		*(.stack)
   	}
}

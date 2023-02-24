ENTRY(kernel_init)

EXTERN(multiboot2_header_start)
EXTERN(gdt)

SECTIONS {
	/* . = 0x800;
	.gdt : {
		KEEP(*(.gdt))
	} */

	. = 1M;
	.boot : ALIGN(64) {
		KEEP(*(.multiboot2_header))
	}

	. = 2M;
	.text : {
		*(.text .text.*)
	}

}

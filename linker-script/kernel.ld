ENTRY(__start)

vm_offset = 0xc0000000;

SECTIONS {
	. = 1M; /* VMA == LMA */

	.boot : ALIGN(64) {
		*(.boot)
		__boot_end = ALIGN(4K);
	}

	. = vm_offset + __boot_end; /* VMA != LMA */

	.text : AT(__boot_end) {
		*(.text .text.*)
	}

	.data : ALIGN(4K) {
		*(.data .data.*)
		*(.rodata .rodata.*)
	}

	.bss : ALIGN(4K) {
		*(.bss .bss.*)
		. = ALIGN(4K);
		*(.stack)
	}
}

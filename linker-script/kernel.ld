ENTRY(__start)

vm_offset = 0xc0000000;
GLOBAL_PD_VIRT = vm_offset + GLOBAL_PD;

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

	.test_array : ALIGN(4K) {
		__test_array_start = .;
		KEEP(*(.test_array))
		__test_array_end = .;
	}

	.bss : ALIGN(4K) {
		*(.bss .bss.*)
		. = ALIGN(4K);
		*(.stack)
	}

	.trampoline : ALIGN(4K) {
		__trampoline_start = .;
		*(.trampoline .trampoline.*)
		. = ALIGN(4K);
		__trampoline_end = .;
	}
}

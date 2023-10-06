ENTRY(_start)

text_begin = 0x08048000;

SECTIONS
{
	.text text_begin : {
		code = .;
		*(.entry)
		*(.text)
		*(.rodata)
		. = ALIGN(4096);
	}

	.data : {
		data = .;
		*(.data)
		. = ALIGN(4096);
	}

	.bss : {
		bss = .;
		*(.bss)
		. = ALIGN(4096);
	}
	end = .;
}
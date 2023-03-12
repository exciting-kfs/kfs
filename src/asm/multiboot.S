%define MAGIC    0xE85250D6
%define ARCH     0x0 ; x86
%define LENGTH   multiboot2_header_end - multiboot2_header_start
%define CHECKSUM - (ARCH + MAGIC + LENGTH)

section .multiboot2_header
global multiboot2_header_start

multiboot2_header_start:
; multiboot2 required fields.
	dd MAGIC
	dd ARCH
	dd LENGTH
	dd CHECKSUM
; aditional tags starts from HERE.

; END of TAGS.
	dw 0
	dw 0
	dd 8
multiboot2_header_end:

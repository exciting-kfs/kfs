%include "segment.S"

section .text

global load_flat_gdt:function (load_flat_gdt.end - load_flat_gdt)
load_flat_gdt:
	lgdt [gdtr]
	; Reload CS via long jump
	jmp KERNEL_CODE:.reload_data_segment
.reload_data_segment:
	mov ax, KERNEL_DATA
	mov ds, ax
	mov es, ax
	mov fs, ax
	mov gs, ax
	mov ss, ax
	ret
.end:

; Index of each entry is hard coded.
gdt:
.start:
	.null:
		dq 0x0000000000000000
	.kernel_code:
		dq 0x00cf9a000000ffff
	.kernel_data:
		dq 0x00cf92000000ffff
	.user_code:
		dq 0x00cffa000000ffff
	.user_data:
		dq 0x00cff2000000ffff
.end:
gdtr:
	dw gdt.end - gdt.start - 1
	dd gdt.start
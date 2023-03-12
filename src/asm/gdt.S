section .gdt

gdt_start:
; null descriptor
	dq 0x0000000000000000
; KERNEL CODE
kernel_code_entry:
	dq 0x00cf9a000000ffff
; KERNEL DATA
kernel_data_entry:
	dq 0x00cf92000000ffff
; USER CODE
user_code_entry:
	dq 0x00cffa000000ffff
; USER DATA
user_data_entry:
	dq 0x00cff2000000ffff
gdt_end:

section .text
global load_flat_gdt

gdtr_pointer:
	dw gdt_end - gdt_start - 1
	dd gdt_start

load_flat_gdt:
    lgdt [gdtr_pointer]
	; reload CS via long jump
	jmp (kernel_code_entry - gdt_start):reload_data_segment

reload_data_segment:
    mov ax, (kernel_data_entry - gdt_start)
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
	ret
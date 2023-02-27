SEG_KERNEL_CODE equ 0x8
SEG_KERNEL_DATA equ 0x10
SEG_KERNEL_STACK equ 0x18
SEG_USER_CODE equ 0x23
SEG_USER_DATA equ 0x2b
SEG_USER_STACK equ 0x33

GDTR equ 0x800 - 0x6

section .text.gdt
extern enter_rust_code
global gdt_init
gdt_init:
	mov eax, cr0
	xor eax, 0x01	; turn off PE flag
	mov cr0, eax	; enter real mode

	lgdt [GDTR]

	mov eax, cr0
	or eax, 0x01	; turn on PE flag
	mov cr0, eax	; enter protected mode

	mov eax, SEG_KERNEL_DATA
	mov ds, eax
	mov es, eax
	mov fs, eax
	mov gs, eax

	mov eax, SEG_KERNEL_STACK
	mov ss, eax

	mov esp, 0x9fffe0

	jmp SEG_KERNEL_CODE:enter_rust_code
SEG_KERNEL_CODE equ 0x8
SEG_KERNEL_DATA equ 0x10
SEG_KERNEL_STACK equ 0x18
SEG_USER_CODE equ 0x23
SEG_USER_DATA equ 0x2b
SEG_USER_STACK equ 0x33
stack_init equ 0xefffe0

GDTR equ 0x800 - 0x6

section .text.gdt
extern enter_rust_code
global gdt_init
global stack_init
gdt_init:
	mov ecx, cr0
	xor ecx, 0x01	; turn off PE flag
	mov cr0, ecx	; enter real mode

	lgdt [GDTR]

	mov ecx, cr0
	or ecx, 0x01	; turn on PE flag
	mov cr0, ecx	; enter protected mode

	mov ecx, SEG_KERNEL_DATA
	mov ds, ecx
	mov es, ecx
	mov fs, ecx
	mov gs, ecx

	mov ecx, SEG_KERNEL_STACK
	mov ss, ecx

	mov esp, stack_init
	mov ebp, esp
	
	jmp SEG_KERNEL_CODE:enter_rust_code
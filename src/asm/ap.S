%include "address.S"

section .text

global relocate_ap_start
global __ap_start

extern __ap_stack_allocate
extern AP_START

extern AP_COUNT

extern enable_paging_ap

extern GLOBAL_PD

extern kernel_caller_ret
extern kernel_stack_bottom

extern ap_entry

; In MP spec, B.4.2,
; `The STARTUP IPI causes the target processor to start executing in Real Mode
; from address 000VV000h, where VV is an 8-bit vector that is part of the IPI message.`
BITS 16

global __ap_start
__ap_start:
	cli
	lgdt [__gdtr - __ap_start + AP_START]
	mov eax, cr0
	or eax, 1
	mov cr0, eax
	jmp 8:(__ap_init - __ap_start + AP_START)
BITS 32
__ap_init:
	mov ax, 16
	mov ds, ax
	mov es, ax
	mov fs, ax
	mov gs, ax
	mov ss, ax

	mov eax, 0xb8001
	mov BYTE [eax], 65

	; set temp stack
	mov esp, AP_START
	mov ebp, esp

	call __ap_enable_paging ; mapping problem

	
	; raise AP_COUNT
	xor ebx, ebx
	lea eax, AP_COUNT
	mov bl, BYTE [eax]
	inc bl
	mov BYTE [eax], bl

	jmp $

	; allocate stack for ap
	mov eax, __ap_stack_allocate
	call eax
	sub eax, VM_OFFSET
	mov esp, eax
	mov ebp, esp



	; set for stack trace (?)
	push kernel_caller_ret
	push kernel_stack_bottom
	push ebx ; (physical id)


	call ap_entry
__ap_enable_paging:
	; Enable PSE and PGE
	mov eax, cr4
	or eax, 0x90; PSE | PGE
	mov cr4, eax

	; Load global PD into CR3
	mov eax, GLOBAL_PD
	mov cr3, eax

	; Enable paging
	mov eax, cr0
	or eax, 0x80000000; PG
	mov cr0, eax

	; translate registers and return.
	add ebp, VM_OFFSET
	add esp, VM_OFFSET
	add DWORD [esp], VM_OFFSET
	ret
__gdt:
	dq 0x0000000000000000
__code:
	dq 0x00cf9a000000ffff
__kdata:
	dq 0x00cf92000000ffff
__gdt_end:
__gdtr:
	dw __gdt_end - __gdt - 1
	dd __gdt - __ap_start + AP_START
global __ap_end
__ap_end:


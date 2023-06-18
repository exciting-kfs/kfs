%include "address.S"

; global, PS(4M), kernel, write, present.
%define PD_ATTRIBUTE 0b000110000011

section .text

extern AP_START
extern AP_COUNT
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

	call __ap_enable_paging

	; raise AP_COUNT
	xor ebx, ebx
	lea eax, AP_COUNT
	mov bl, BYTE [eax]
	inc bl
	mov BYTE [eax], bl

	; set for stack trace (?)
	push kernel_caller_ret
	push kernel_stack_bottom
	push ebx ; (physical id): physical id and AP_COUNT is identical.

	lea eax, ap_entry
	call eax
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


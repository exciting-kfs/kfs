%include "address.S"

section .text

global relocate_ap_start
global __ap_start

extern AP_START

extern AP_FLAG

; In MP spec, B.4.2,
; `The STARTUP IPI causes the target processor to start executing in Real Mode
; from address 000VV000h, where VV is an 8-bit vector that is part of the IPI message.`
__ap_start:
	mov bx, 0xb800
	mov ds, bx
	mov ax, 65
	mov [ds:0x0001], ax

	mov eax, cr0
	or eax, 0x1
	mov cr0, eax

	lea eax, AP_FLAG
	mov byte [eax], 0x1

	jmp $
.end:

relocate_ap_start:
	mov edi, virtual_addr(AP_START)
	mov esi, __ap_start
	mov ecx, (__ap_start.end - __ap_start)
	rep movsb

	xor eax, eax
	mov ecx, 0x100
	rep stosb

	ret

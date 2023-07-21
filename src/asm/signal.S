section .text

extern return_from_interrupt

global go_to_signal_handler
go_to_signal_handler:
	mov ebp, esp
	mov eax, [ebp + 0x04]   ; interrupt frame at kstack bottom.

	push dword [eax + 0x44] ; ss
	push dword [ebp + 0x08] ; new_esp
	push dword [eax + 0x3c] ; eflags
	push dword [eax + 0x38] ; cs
	push dword [ebp + 0x0c] ; eip: siganl_handler
	push 0                  ; intr_handler
	push 0                  ; error code

	push dword [eax + 0x28] ; gs
	push dword [eax + 0x24] ; fs
	push dword [eax + 0x20] ; es
	push dword [eax + 0x1c] ; ds
	push 0                  ; eax
	push 0                  ; ebx
	push 0                  ; ecx
	push 0                  ; edx
	push 0                  ; esi
	push 0                  ; edi
	push dword [eax]        ; ebp

	jmp return_from_interrupt

global return_from_signal_handler
return_from_signal_handler:
	mov eax, [esp] ; sig_ctx

	mov ebx, [eax]
	mov edi, [eax + 0x04]
	mov esi, [eax + 0x08]
	mov ebp, [eax + 0x0c]

	mov esp, [esp + 4] ; old_esp
	jmp [eax + 0x10]

section .trampoline

global signal_trampoline:function (signal_trampoline.end - signal_trampoline)
signal_trampoline:
	; mov ecx, [esp + 4]  ; sig_info
	mov ebx, [esp + 8]  ; sig_context
	mov eax, 0x77       ; sigreturn
	int 0x80
.end:
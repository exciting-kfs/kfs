%include "common.S"

; INTERRUPT_HANDLER(name: ident, callback: ident, has_error_code: bool)
; define new interrupt handler.
; 
; - name: new handler's name
; - callback: after saving states, this callback will be called.
; - has_error_code: dummy error_code will pushed based on this argument.
%macro INTERRUPT_HANDLER 3
    extern %2
    global %1
    %1:
        %if !%3
            push 0
        %endif
        push %2
        jmp handle_interrupt
%endmacro

section .text

INTERRUPT_HANDLER handle_timer, handle_timer_impl, FALSE
INTERRUPT_HANDLER handle_keyboard, handle_keyboard_impl, FALSE
INTERRUPT_HANDLER handle_divide_error, handle_divide_error_impl, FALSE
INTERRUPT_HANDLER handle_invalid_opcode, handle_invalid_opcode_impl, FALSE
INTERRUPT_HANDLER handle_general_protection, handle_general_protection_impl, TRUE
INTERRUPT_HANDLER handle_page_fault, handle_page_fault_impl, TRUE

global handle_interrupt
handle_interrupt:
    ; backup additional registers
    push gs
    push fs
    push es
    push ds
    push eax
    push ebx
    push ecx
    push edx
    push esi
    push edi
    push ebp

    ; load kernel data segments
    mov ax, 16
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    call [esp + 44] ; call real handler function

    pop ebp
    pop edi
    pop esi
    pop edx
    pop ecx
    pop ebx
    pop eax
    pop ds
    pop es
    pop fs
    pop gs

    add esp, 8 ; pop error code and handler

    iret

struc kctx
    .eip: resd 1
    .esp: resd 1
endstruc

global switch_process
switch_process:
    ; ax cx dx are caller-saved registers
    push edi
    push esi

    mov esi, [esp + 12]
    mov edi, [esp + 16]

    mov DWORD [esi + kctx.esp], esp
    mov esp, [edi + kctx.esp]

    mov DWORD [esi + kctx.eip], switch_end
    push DWORD [edi + kctx.eip]

    ret
switch_end:
    pop esi
    pop edi

    ret
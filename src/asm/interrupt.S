%include "segment.S"
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

INTERRUPT_HANDLER handle_timer,                 handle_timer_impl,              FALSE
INTERRUPT_HANDLER handle_keyboard,              handle_keyboard_impl,           FALSE
INTERRUPT_HANDLER handle_divide_error,          handle_divide_error_impl,       FALSE
INTERRUPT_HANDLER handle_invalid_opcode,        handle_invalid_opcode_impl,     FALSE
INTERRUPT_HANDLER handle_general_protection,    handle_general_protection_impl, TRUE
INTERRUPT_HANDLER handle_page_fault,            handle_page_fault_impl,         TRUE

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
    mov ax, KERNEL_DATA
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

global switch_stack
switch_stack:
    ; save callee-saved registers
    push ebp
    push esi
    push edi
    push ebx

    mov esi, [esp + 24] ; next_stack
    mov edi, [esp + 20] ; prev_stack

    ; do stack switching with atomic operations.
    lfence
    xchg [edi], esp
    xchg esp, [esi]
    sfence

    pop ebx
    pop edi
    pop esi
    pop ebp

    ret

global kthread_exec
kthread_exec:
    mov esp, [esp + 4]
    add esp, 16
    ret

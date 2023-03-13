%define STACK_SIZE 0x400000 ; 4MB

extern kernel_entry
extern load_flat_gdt

section .kernel_stack nobits ; uninitialized section.
global kernel_stack_init
    resb STACK_SIZE
kernel_stack_init:

section .text
global kernel_init

; represent caller of `kernel_init` who doesn't really exists.
; this needed for stack trace.
kernel_caller:
    call kernel_init
kernel_caller_ret:

; kernel image entry point.
kernel_init:
    ; initialize kernel stack.
    mov esp, kernel_stack_init - 0x10
    mov ebp, esp
    ; create virtual caller's stack frame
    mov DWORD [esp], kernel_stack_init
    mov DWORD [esp + 4], kernel_caller_ret

    ; prepare arguments for `kernel_entry`.
    sub esp, 0x10
    mov [esp], ebx      ; (boot_info)
    mov [esp + 4], eax  ; (boot_magic)

    ; load custom gdt and reload segment registers.
    call load_flat_gdt

    call kernel_entry
    ; unreachable.
%include "address.S"

%define STACK_SIZE 0x400000; 4MB FIXME: too large?

extern enable_paging
extern kernel_entry
extern load_flat_gdt

extern relocate_ap_start
extern __ap_start

section .stack nobits write
global kernel_stack_bottom
    resb STACK_SIZE
kernel_stack_bottom:

section .text
global kernel_init:function (kernel_init.end - kernel_init)

global kernel_caller_ret
; Represent caller of `kernel_init` who doesn't really exists.
; this needed for stack trace.
kernel_caller:
    call kernel_init
kernel_caller_ret:

; Perform basic initialization. such as prepare stack, enable paging, ...
; and jump to RUST part.
kernel_init:
    ; Initialize kernel stack.
    mov esp, physical_addr(kernel_stack_bottom) - 0x10
    mov ebp, esp

    ; Create virtual caller's stack frame
    mov DWORD [esp], kernel_stack_bottom
    mov DWORD [esp + 4], kernel_caller_ret

    ; Prepare arguments for `kernel_entry`.
    sub esp, 0x10
    add ebx, VM_OFFSET
    mov [esp], ebx ; (boot_info)
    mov [esp + 4], eax ; (boot_magic)

    ; Enable paging and translate EIP, ESP, EBP to virtual address.
    call enable_paging

    ; Load GDTR and reload segment registers.
    call load_flat_gdt

    ; Kernel main logic. (no return)
    call kernel_entry
; Unreachable. but just in case...
.loop:
    cli
    hlt
    jmp .loop
.end:


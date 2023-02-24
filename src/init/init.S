section .text
extern kernel_entry
extern gdt_init
global kernel_init
global enter_rust_code

for_stacktrace:
    call kernel_init
kernel_init:
    mov esp, stack_init
    mov ebp, esp
    push kernel_init
    push ebp
    mov ebp, esp
    
    push eax
    push ebx
    mov al, 0xa7
    out 0x64, al
    call kernel_entry

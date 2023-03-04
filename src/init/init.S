%define STACK_SIZE 0x400000 ; 4MB

section .stack
global stack_init
    resb STACK_SIZE
stack_init:

section .text
extern kernel_entry
global kernel_init

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

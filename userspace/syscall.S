%define EXIT_ENTRY 1
%define FORK_ENTRY 2
%define READ_ENTRY 3
%define WRITE_ENTRY 4
%define WAITPID_ENTRY 7

section .text

global fork
fork:
    mov eax, FORK_ENTRY
    int 0x80
    ret

global read
read:
    mov eax, READ_ENTRY
    mov ebx, [esp + 4]
    mov ecx, [esp + 8]
    mov edx, [esp + 12]
    int 0x80
    ret

global write
write:
    mov eax, WRITE_ENTRY
    mov ebx, [esp + 4]
    mov ecx, [esp + 8]
    mov edx, [esp + 12]
    int 0x80
    ret

global fortytwo
fortytwo:
    mov eax, 42
    mov ebx, [esp + 4]
    int 0x80
    ret

global _exit
_exit:
    mov eax, EXIT_ENTRY
    mov ebx, [esp + 4]
    int 0x80
    ret

global waitpid
waitpid:
    mov eax, WAITPID_ENTRY 
    mov ebx, [esp + 4]
    mov ecx, [esp + 8]
    mov edx, [esp + 12]
    int 0x80
    ret
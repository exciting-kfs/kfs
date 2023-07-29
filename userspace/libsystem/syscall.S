%define EXIT_ENTRY 1
%define FORK_ENTRY 2
%define READ_ENTRY 3
%define WRITE_ENTRY 4
%define EXEC_ENTRY 11 ; TODO: execve
%define SIGNAL_ENTRY 48
%define SIGACTION_ENTRY 67

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

global signal
signal:
    mov eax, SIGNAL_ENTRY
    mov ebx, [esp + 4]
    mov ecx, [esp + 8]
    int 0x80
    ret

global sigaction
sigaction:
    mov eax, SIGACTION_ENTRY
    mov ebx, [esp + 4]
    mov ecx, [esp + 8]
    mov edx, [esp + 12]
    int 0x80
    ret

global _exit
_exit:
    mov eax, EXIT_ENTRY
    mov ebx, [esp + 4]
    int 0x80
    ret

global exec
exec:
    mov eax, EXEC_ENTRY
    mov ebx, [esp + 4]
    int 0x80
    ret

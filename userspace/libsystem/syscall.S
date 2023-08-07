; SYSCALL(name: ident, vector_number: int, nr_arguments: int)
; define new syscall
%macro SYSCALL 3
    global %1
    %1:
        push ebp
        mov eax, %2
        %if %3 > 0
            mov ebx, [esp + 8]
        %endif
        %if %3 > 1
            mov ecx, [esp + 12]
        %endif
        %if %3 > 2
            mov edx, [esp + 16]
        %endif
        %if %3 > 3
            mov esi, [esp + 20]
        %endif
        %if %3 > 4
            mov edi, [esp + 24]
        %endif
        %if %3 > 5
            mov ebp, [esp + 28]
        %endif
        int 0x80
        pop ebp
        ret
%endmacro

section .text

;         name           vector number   arguments
SYSCALL _exit,          1,              1
SYSCALL fork,           2,              0
SYSCALL read,           3,              3
SYSCALL write,          4,              3
SYSCALL open,           5,              0 ; FIXME
SYSCALL waitpid,        7,              3
SYSCALL exec,           11,             1
SYSCALL getpid,         20,             0
SYSCALL kill,           37,             2
SYSCALL pipe,           42,             1
SYSCALL signal,         48,             2
SYSCALL setpgid,        57,             2
SYSCALL getppid,        64,             0
SYSCALL getpgrp,        65,             0
SYSCALL setsid,         66,             0
SYSCALL sigaction,      67,             3
SYSCALL getpgid,        132,            1
SYSCALL getsid,         147,            0
SYSCALL sched_yield,    158,            0
SYSCALL mmap,           192,            6
SYSCALL getuid,         199,            0
SYSCALL setuid,         213,            1

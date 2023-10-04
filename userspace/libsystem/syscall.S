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
SYSCALL open,           5,              3
SYSCALL close,          6,              1
SYSCALL waitpid,        7,              3
SYSCALL creat,          8,              2
SYSCALL unlink,         10,             1
SYSCALL execve,         11,             5
SYSCALL chdir,          12,             1
SYSCALL chmod,          15,             2
SYSCALL stat,           18,             2
SYSCALL lseek,          19,             3
SYSCALL getpid,         20,             0
SYSCALL mount,          21,             2 ; FIXME
SYSCALL umount,         22,             1
SYSCALL kill,           37,             2
SYSCALL mkdir,          39,             2
SYSCALL rmdir,          40,             1
SYSCALL pipe,           42,             1
SYSCALL signal,         48,             2
SYSCALL setpgid,        57,             2
SYSCALL getppid,        64,             0
SYSCALL getpgrp,        65,             0
SYSCALL setsid,         66,             0
SYSCALL sigaction,      67,             3
SYSCALL symlink,        83,             2
SYSCALL mmap,           90,             6
SYSCALL munmap,         91,             2
SYSCALL truncate,       92,             2
SYSCALL getpgid,        132,            1
SYSCALL getdents,       141,            3
SYSCALL getsid,         147,            0
SYSCALL sched_yield,    158,            0
SYSCALL getcwd,         183,            2
SYSCALL getuid,         199,            0
SYSCALL getgid,         200,            0
SYSCALL chown,          212,            3
SYSCALL setuid,         213,            1
SYSCALL setgid,         214,            1
SYSCALL socket,         359,            3
SYSCALL bind,           361,            3
SYSCALL connect,        362,            3
SYSCALL listen,         363,            2
SYSCALL accept,         364,            3
SYSCALL sendto,         369,            5
SYSCALL recvfrom,       371,            5

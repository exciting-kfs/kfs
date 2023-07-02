section .text

global user_start
user_start:

xor eax, eax
.busy_wait:
    inc eax
    cmp eax, 1000000000
    jne .busy_wait ; jmp short
mov eax, 42
int 0x80
jmp user_start ; jmp short

global user_end
user_end:

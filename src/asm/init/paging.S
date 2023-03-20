%include "address.S"

; currently global page directory is located at 0x100000. (1M)
%define GLOBAL_PD 0x100000

; first 4 PDE (16MB) will be marked as present
%define INIT_DIRS 4

; global, PS(4M), user, write, present.
%define PD_ATTRIBUTE 0b110000111

%define VM_PD_OFFSET (VM_OFFSET / (1024 * 1024))

section .text
global enable_paging:function (enable_paging.end - enable_paging)

; enable paging.
;
; more specifically,
; 1) enable following paging extensions.
;     - PGE(page global extension)
;     - PSE(page size extension)
; 2) initialize global page directory.
; 3) load CR3 with global age directory.
; 4) enable paging.
; 5) translate EIP, ESP, EBP to virtual address.
;
; after all,
; physical memory address [0, size) (where size is `4 * INIT_DIRS` megabytes)
; will be mapped virtual memory address [0, size) and [VM_OFFSET,  VM_OFFSET + size).
enable_paging:
    ; clear global PD
    xor eax, eax
    mov ecx, 1024
    mov edi, GLOBAL_PD
    cld
    rep stosd

    ; flat-map beginning part of memory
    mov ebx, GLOBAL_PD
    xor ecx, 0
    .loop:
        cmp ecx, INIT_DIRS
        je .loop_end

        ; prepare each entry
        mov eax, ecx
        shl eax, 22
        or eax, PD_ATTRIBUTE

        mov DWORD [ebx + (ecx * 4)], eax
        mov DWORD [ebx + (ecx * 4) + VM_PD_OFFSET], eax

        inc ecx
        jmp .loop
    .loop_end:

    ; Enable PSE and PGE
    mov eax, cr4
    or eax, 0x90; PSE | PGE
    mov cr4, eax

    ; Load global PD into CR3
    mov eax, GLOBAL_PD
    mov cr3, eax

    ; Enable paging
    mov eax, cr0
    or eax, 0x80000000; PG
    mov cr0, eax

    ; translate registers and return.
    add ebp, VM_OFFSET
    add esp, VM_OFFSET
    add DWORD [esp], VM_OFFSET
    ret

.end:
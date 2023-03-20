%include "address.S"

%define MAGIC    0xE85250D6
%define ARCH     0x0 ; x86
%define LENGTH   multiboot2_header_end - multiboot2_header_start
%define CHECKSUM - (ARCH + MAGIC + LENGTH)

extern kernel_init

; Unlike other sections, this section has same LMA and VMA.
; Since the entry point must defined at physical address,
; we have to put entry point in this section.
section .boot exec

global global_pd:data 0x1000
global global_first_pt:data 0x1000

global __start:function (__start.end - __start)

; Initially, this space holds multiboot2 header.
; But during booting sequence,
; header will be overwritten with global page directory.
global_pd:
multiboot2_header_start:
	dd MAGIC
	dd ARCH
	dd LENGTH
	dd CHECKSUM
	; --- Extra  TAGS ---
	; --- End of TAGS ---
	dw 0
	dw 0
	dd 8
multiboot2_header_end:

; Just jump to real entry point.
; TODO: enforce absolute call.
__start:
	jmp physical_addr(kernel_init)
.end:

; In order to prevent another section is placed here
; ([global_pd, global_pd + 1page)) by linker or bootloader,
; allocate at least 1 page explictly.
times 0x1000 - $ + global_pd db 0

; This space will be used as first page table.
; which holds null-page and low-address MMIO pages.
global_first_pt:
times 0x1000 db 0

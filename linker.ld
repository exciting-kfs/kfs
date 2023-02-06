ENTRY(kernel_entry)

SECTIONS {
    . = 1M;
    .boot : ALIGN(64) {
        *(.multiboot2_header)
    }
    .text : {
        *(.text.*)
    }
}

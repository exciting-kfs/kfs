ENTRY(kernel_entry)

SECTIONS {
    . = 1M;
    .boot :{ *(.multiboot2_header) }
    .text :{ *(.text) }
}

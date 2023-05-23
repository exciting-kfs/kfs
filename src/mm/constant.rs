pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);

pub const PT_ENTRIES: usize = 1024;
pub const PD_ENTRIES: usize = 1024;
pub const PT_COVER_SIZE: usize = PT_ENTRIES * PAGE_SIZE;

pub const VM_OFFSET: usize = 0xc000_0000;
pub const VMALLOC_OFFSET: usize = 0xf800_0000;

pub const KB: usize = 1024;
pub const MB: usize = 1024 * KB;

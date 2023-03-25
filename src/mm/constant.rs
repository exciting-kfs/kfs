pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;

pub const PT_ENTRIES: usize = 1024;
pub const PD_ENTRIES: usize = 1024;
pub const PT_COVER_SIZE: usize = PT_ENTRIES * PAGE_SIZE;

pub const VM_OFFSET: usize = 0xc0_00_00_00;
pub const ZONE_NORMAL: usize = 768;

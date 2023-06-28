pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);

pub const PT_ENTRIES: usize = 1024;
pub const PD_ENTRIES: usize = 1024;
pub const PT_COVER_SIZE: usize = PT_ENTRIES * PAGE_SIZE;

pub const VM_OFFSET: usize = 0xc000_0000;
pub const VMALLOC_OFFSET: usize = 0xf800_0000;

/// 0x1_0000_0000 / PAGE_SIZE
pub const LAST_PFN: usize = (0x8000_0000 / PAGE_SIZE) + (0x8000_0000 / PAGE_SIZE);

pub const MAX_RANK: usize = 10;

/// Maximum allocation size per request.
pub const BLOCK_SIZE: usize = PAGE_SIZE * (1 << MAX_RANK);

/// SIZE
pub const KB: usize = 1024;
pub const MB: usize = 1024 * KB;

/// cache allocator
pub const LEVEL_MIN: usize = 6;
pub const LEVEL_END: usize = 12;
pub const LEVEL_RNG: usize = LEVEL_END - LEVEL_MIN;

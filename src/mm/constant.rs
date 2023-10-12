use super::util::multiplier_bigger_than;

pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);

pub const PT_ENTRIES: usize = 1024;
pub const PD_ENTRIES: usize = 1024;
pub const PT_COVER_SIZE: usize = PT_ENTRIES * PAGE_SIZE;

// Virtual address space
// OFFSET 0         3072MB          3972MB           4072MB      4076MB          4096MB
//        [ USER(3GB) | KERNEL(896MB) | VMALLOC(104MB) | KMAP(4MB) | HIGH_IO(20MB) ]
pub const VM_OFFSET: usize = 0xc000_0000;
pub const VMALLOC_OFFSET: usize = 0xf800_0000;
pub const KMAP_OFFSET: usize = 0xfe800000;
pub const HIGH_IO_OFFSET: usize = 0xfec00000;

/// 0x1_0000_0000 / PAGE_SIZE
pub const LAST_PFN: usize = (0x8000_0000 / PAGE_SIZE) + (0x8000_0000 / PAGE_SIZE);

pub const MAX_RANK: usize = 10;

/// Maximum allocation size per request.
pub const BLOCK_SIZE: usize = PAGE_SIZE * (1 << MAX_RANK);

/// SIZE
pub const KB: usize = 1024;
pub const MB: usize = 1024 * KB;
pub const GB: usize = 1024 * MB;
pub const SECTOR_SIZE: usize = 512;

/// cache allocator
pub const MAX_CAHCE_SIZE: usize = 2048;
pub const MIN_CAHCE_SIZE: usize = 64;
pub const MIN_CACHE_SIZE_MULTIPLIER: usize = multiplier_bigger_than(MIN_CAHCE_SIZE);
pub const MAX_CACHE_SIZE_MULTIPLIER: usize = multiplier_bigger_than(MAX_CAHCE_SIZE);
pub const NR_CACHE_ALLOCATOR: usize = MAX_CACHE_SIZE_MULTIPLIER - MIN_CACHE_SIZE_MULTIPLIER + 1;
pub const MAX_CACHE_PAGE_PER_ALLOCATOR: usize = GB / NR_CACHE_ALLOCATOR;

/// OOM
const OOM_WATER_MARK_BYTE: usize = 100 * MB;
pub const OOM_WATER_MARK: usize = OOM_WATER_MARK_BYTE / PAGE_SIZE;

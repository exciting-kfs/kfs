pub const NR_CPUS: usize = 4;

pub const KSTACK_RANK: usize = 10;

pub const USTACK_PAGES: usize = 256; // 1MB
pub const USTACK_BASE: usize = 0xc000_0000;

pub const MAX_PAGE_PER_ARG: usize = 1;
pub const MAX_PAGE_PER_ARGV: usize = 32;

pub const TRAMPOLINE_BASE: usize = 0xa000_0000;

pub const TIMER_FREQUENCY_HZ: usize = 250;
pub const NR_CONSOLES: usize = 4;

pub const PATH_MAX: usize = 1024;

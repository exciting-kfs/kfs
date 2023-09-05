pub const NR_CPUS: usize = 4;

pub const KSTACK_RANK: usize = 10;

pub const USTACK_PAGES: usize = 256; // 1MB
pub const USTACK_BASE: usize = 0xc000_0000;
pub const USER_CODE_BASE: usize = 0x0804_8000;
pub const TRAMPOLINE_BASE: usize = 0xa000_0000;

pub const TIMER_FREQUENCY_HZ: usize = 1000;
pub const CONSOLE_COUNTS: usize = 4;

pub const PATH_MAX: usize = 256;

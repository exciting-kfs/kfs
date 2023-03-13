pub const PAGE_ENTRY_COUNT: usize = 1024;
pub const PAGE_ADDR_LSB: usize = 12;
pub const BIT_GLOBAL: usize = 8;
pub const BIT_PAGESIZE: usize = 7;
pub const BIT_PAT: usize = 7;
pub const BIT_DIRTY: usize = 6;
pub const BIT_ACCESSED: usize = 5;
pub const BIT_PCD: usize = 4;
pub const BIT_PWT: usize = 3;
pub const BIT_PRIVILEGE: usize = 2;
pub const BIT_RW: usize = 1;
pub const BIT_PRESENT: usize = 0;


pub fn get_bit(data: usize, nth_bit: usize) -> bool {
	(data >> nth_bit) & 0x1 == 0x1
}
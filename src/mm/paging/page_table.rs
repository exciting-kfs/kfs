use super::constants::{
	get_bit,
	PAGE_ENTRY_COUNT,
	PAGE_ADDR_LSB,
	BIT_ACCESSED,
	BIT_PCD,
	BIT_PRESENT,
	BIT_PRIVILEGE,
	BIT_PWT,
	BIT_RW,
	BIT_PAT,
	BIT_GLOBAL,
	BIT_DIRTY
};

#[repr(packed)]
pub struct PageTable {
	entry: [PageTabEntryPacked; PAGE_ENTRY_COUNT]
}

impl PageTable {
	pub const fn new() -> Self {
		PageTable { entry: [PageTabEntryPacked::new(); PAGE_ENTRY_COUNT] } 
	}
}

#[derive(Clone, Copy)]
#[repr(packed)]
struct PageTabEntryPacked {
	data: usize
}

struct PageTabEntry {
	addr: *const usize,
	global: bool,
	pat: bool,		// page attribute table
	dirty: bool,	
	accessed: bool,
	pcd: bool,		// page-level cache disable.
	pwt: bool,		// page-level write-through.
	privilege: bool,	// kerenl: false, user: true
	rw: bool,		// read-only: false, writable: true
	present: bool,
}

impl PageTabEntryPacked {
	pub const fn new() -> Self {
		PageTabEntryPacked {
			data: 0
		}
	}

	pub fn unpack(self) -> PageTabEntry {
		let data = self.data;

		PageTabEntry {
			addr: (data >> PAGE_ADDR_LSB) as *const usize,
			global: get_bit(data, BIT_GLOBAL),
			pat: get_bit(data, BIT_PAT),
			dirty: get_bit(data, BIT_DIRTY),
			accessed: get_bit(data, BIT_ACCESSED),
			pcd: get_bit(data, BIT_PCD),
			pwt: get_bit(data, BIT_PWT),
			privilege: get_bit(data, BIT_PRIVILEGE),
			rw: get_bit(data, BIT_RW),
			present: get_bit(data, BIT_PRESENT)
		}
	}
}

impl PageTabEntry {
	pub fn pack(self) -> PageTabEntryPacked {
		let addr = self.addr as usize;
		let data = {
			addr << PAGE_ADDR_LSB
			+ (self.global as usize) << BIT_GLOBAL
			+ (self.pat as usize) << BIT_PAT
			+ (self.dirty as usize) << BIT_DIRTY
			+ (self.accessed as usize) << BIT_ACCESSED
			+ (self.pcd as usize) << BIT_PCD
			+ (self.pwt as usize) << BIT_PWT
			+ (self.privilege as usize) << BIT_PRIVILEGE
			+ (self.rw as usize) << BIT_RW
			+ self.present as usize
		};
		PageTabEntryPacked { data }
	}
}
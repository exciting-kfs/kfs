use super::constants::{
	PAGE_ENTRY_COUNT,
	PAGE_ADDR_LSB,
	BIT_ACCESSED,
	BIT_PAGESIZE,
	BIT_PCD,
	BIT_PRESENT,
	BIT_PRIVILEGE,
	BIT_PWT,
	BIT_RW,
	get_bit
};

pub static KERNEL_PAGE_DIRECTORY : PageDirectory = PageDirectory::new();

#[repr(packed)]
pub struct PageDirectory {
	entry: [PageDirEntryPacked; PAGE_ENTRY_COUNT]
}

impl PageDirectory {
	pub const fn new() -> Self {
		PageDirectory {
			entry: [PageDirEntryPacked::new(); PAGE_ENTRY_COUNT]
		}
	}
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct PageDirEntryPacked {
	data: usize
}

impl PageDirEntryPacked {
	pub const fn new() -> Self {
		PageDirEntryPacked {
			data: 0
		}
	}

	pub fn unpack(self) -> PageDirEntry {
		let data = self.data;

		PageDirEntry {
			addr: (data >> PAGE_ADDR_LSB) as *const usize,
			page_size: get_bit(data, BIT_PAGESIZE),
			accessed: get_bit(data, BIT_ACCESSED),
			pcd: get_bit(data, BIT_PCD),
			pwt: get_bit(data, BIT_PWT),
			privilege: get_bit(data, BIT_PRIVILEGE),
			rw: get_bit(data, BIT_RW),
			present: get_bit(data, BIT_PRESENT)
		}
	}
}

pub struct PageDirEntry {
	addr: *const usize,
	page_size: bool,	// if CR4.PSE = 1, must be 'false'. if 'true', it means 4MB page operation. 
	accessed: bool,
	pcd: bool,		// page-level cache disable.
	pwt: bool,		// page-level write-through.
	privilege: bool,	// kerenl: false, user: true
	rw: bool,		// read-only: false, writable: true
	present: bool,
}

impl PageDirEntry {
	pub fn pack(self) -> PageDirEntryPacked {
		let addr = self.addr as usize;
		let data = {
			addr << PAGE_ADDR_LSB
			+ (self.page_size as usize) << BIT_PAGESIZE
			+ (self.accessed as usize) << BIT_ACCESSED
			+ (self.pcd as usize) << BIT_PCD
			+ (self.pwt as usize) << BIT_PWT
			+ (self.privilege as usize) << BIT_PRIVILEGE
			+ (self.rw as usize) << BIT_RW
			+ self.present as usize
		};

		PageDirEntryPacked { data }
	}
}

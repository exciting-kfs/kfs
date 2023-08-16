use core::alloc::AllocError;
use core::ops::{Deref, DerefMut};
use core::ptr::addr_of_mut;

use crate::mm::alloc::{page, Zone};
use crate::mm::constant::*;

use super::{PageFlag, PDE};

#[derive(Clone, Copy)]
#[repr(align(4096))]
pub struct PT {
	entries: [PTE; 1024],
}

impl PT {
	pub const fn new() -> Self {
		Self {
			entries: [PTE::empty(); 1024],
		}
	}

	pub fn new_from_4m(pde_4m: PDE) -> Result<&'static mut Self, AllocError> {
		let addr = pde_4m.paddr();
		let flag = pde_4m.flag();
		unsafe {
			let page_table = page::alloc_pages(0, Zone::Normal)?
				.as_mapped()
				.as_mut()
				.as_mut_ptr()
				.cast::<PT>();

			let base = addr_of_mut!((*page_table).entries).cast::<PTE>();

			for i in 0..PT_ENTRIES {
				base.add(i).write(PTE::new(addr + PAGE_SIZE * i, flag));
			}

			return Ok(page_table.as_mut().unwrap());
		}
	}
}

impl Deref for PT {
	type Target = [PTE; 1024];

	fn deref(&self) -> &Self::Target {
		&self.entries
	}
}

impl DerefMut for PT {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.entries
	}
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct PTE {
	data: PageFlag,
}

impl PTE {
	const ADDR_MASK: u32 = 0xffff_f000;

	pub fn new(addr: usize, flags: PageFlag) -> Self {
		Self {
			data: PageFlag::from_bits_retain(addr as u32 & Self::ADDR_MASK) | flags,
		}
	}

	pub const fn empty() -> Self {
		Self {
			data: PageFlag::empty(),
		}
	}

	pub fn set_flag(&mut self, flag: PageFlag) {
		self.data = flag
	}

	pub fn paddr(&self) -> usize {
		(self.data.bits() & Self::ADDR_MASK) as usize
	}

	pub fn flag(&self) -> PageFlag {
		PageFlag::from_bits_truncate(self.data.bits())
	}
}

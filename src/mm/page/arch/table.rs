use core::alloc::AllocError;
use core::array;
use core::ops::{Deref, DerefMut};

use crate::mm::alloc::{page, Zone};
use crate::mm::constant::*;

use super::{PageFlag, PDE};

#[repr(C, align(4096))]
pub struct PT {
	entries: [PTE; 1024],
}

impl PT {
	pub fn new_from_4m(pde_4m: PDE) -> Result<&'static mut Self, AllocError> {
		let addr = pde_4m.addr();
		let flag = pde_4m.flag();
		unsafe {
			let page_table = page::alloc_pages(0, Zone::Normal)?
				.as_mut()
				.as_mut_ptr()
				.cast::<PT>();

			page_table.write(Self {
				entries: array::from_fn(|i| PTE::new(addr + PAGE_SIZE * i, flag)),
			});

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

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PTE {
	data: PageFlag,
}

impl PTE {
	const ADDR_MASK: u32 = 0b11111111_11111111_11110000_00000000;

	pub fn new(addr: usize, flags: PageFlag) -> Self {
		Self {
			data: PageFlag::from_bits_retain(addr as u32 & Self::ADDR_MASK) | flags,
		}
	}

	pub fn set_flag(&mut self, flag: PageFlag) {
		self.data = flag
	}

	pub fn addr(&self) -> usize {
		(self.data.bits() & Self::ADDR_MASK) as usize
	}

	pub fn flag(&self) -> PageFlag {
		PageFlag::from_bits_truncate(self.data.bits())
	}
}

impl AsMut<PageFlag> for PTE {
	fn as_mut(&mut self) -> &mut PageFlag {
		&mut self.data
	}
}

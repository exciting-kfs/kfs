pub mod cast_box;

use core::{alloc::AllocError, mem, ptr::NonNull};

use crate::mm::{
	alloc::{
		page::{alloc_pages, free_pages},
		Zone,
	},
	constant::PAGE_SHIFT,
	util::virt_to_phys,
};

pub struct PageBox {
	ptr: NonNull<[u8]>,
}

impl PageBox {
	pub fn new(zone: Zone) -> Result<Self, AllocError> {
		Self::new_n_ranked(0, zone)
	}

	pub fn new_n_ranked(rank: usize, zone: Zone) -> Result<Self, AllocError> {
		let ptr = alloc_pages(rank, zone)?;

		Ok(Self { ptr })
	}

	pub fn as_virt_ptr(&self) -> *mut u8 {
		self.ptr.as_ptr().cast()
	}

	pub fn as_virt_addr(&self) -> usize {
		self.as_virt_ptr() as usize
	}

	pub fn as_phys_addr(&self) -> usize {
		virt_to_phys(self.as_virt_addr())
	}

	pub fn as_phys_ptr(&self) -> *mut u8 {
		self.as_phys_addr() as *mut u8
	}

	pub fn nr_pages(&self) -> usize {
		self.ptr.len() >> PAGE_SHIFT
	}

	pub fn forget(self) {
		mem::forget(self);
	}
}

impl Drop for PageBox {
	fn drop(&mut self) {
		let page = self.ptr.cast::<u8>();

		free_pages(page);
	}
}

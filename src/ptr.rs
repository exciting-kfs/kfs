use core::{
	alloc::{AllocError, Layout},
	mem,
	ops::{Deref, DerefMut},
	ptr::NonNull,
};

use crate::mm::{
	alloc::{
		page::{alloc_pages, free_pages},
		virt, Zone,
	},
	constant::{PAGE_SHIFT, PAGE_SIZE},
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

pub struct VirtPageBox {
	ptr: NonNull<[u8]>,
	layout: Layout,
}

impl VirtPageBox {
	pub fn new(size: usize) -> Result<Self, AllocError> {
		let layout = unsafe { Layout::from_size_align_unchecked(size, PAGE_SIZE) };
		let ptr = virt::allocate(layout)?;

		Ok(Self { ptr, layout })
	}

	pub fn as_ptr(&self) -> *mut u8 {
		self.ptr.as_ptr().cast()
	}

	pub fn as_slice(&self) -> &[u8] {
		unsafe { &self.ptr.as_ref()[..self.size()] }
	}

	pub fn as_mut_slice(&mut self) -> &mut [u8] {
		unsafe { &mut self.ptr.as_mut()[..self.size()] }
	}

	pub fn size(&self) -> usize {
		self.layout.size()
	}

	pub fn forget(self) {
		mem::forget(self);
	}
}

impl Deref for VirtPageBox {
	type Target = [u8];

	fn deref(&self) -> &Self::Target {
		self.as_slice()
	}
}

impl DerefMut for VirtPageBox {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut_slice()
	}
}

impl Drop for VirtPageBox {
	fn drop(&mut self) {
		let page = self.ptr.cast::<u8>();

		virt::deallocate(page, self.layout);
	}
}

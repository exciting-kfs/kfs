use core::{alloc::AllocError, mem, ptr::NonNull, slice::from_raw_parts_mut};

use crate::mm::{
	alloc::{
		page::{alloc_pages, free_pages},
		Zone,
	},
	constant::{MAX_RANK, PAGE_SHIFT},
	page::index_to_meta,
	util::{addr_to_pfn, phys_to_virt, rank_to_size, virt_to_phys},
};

pub struct PageBox {
	ptr: NonNull<[u8]>,
}

impl PageBox {
	pub fn new(zone: Zone) -> Result<Self, AllocError> {
		Self::new_n_ranked(0, zone)
	}

	pub fn new_n_ranked(rank: usize, zone: Zone) -> Result<Self, AllocError> {
		let ptr = unsafe { alloc_pages(rank, zone)?.as_mapped() }; // ?

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

		free_pages(UnMapped::from_normal(page));
	}
}

#[derive(Clone, Debug)]
pub struct UnMapped {
	paddr: usize,
	rank: usize,
}

impl UnMapped {
	pub fn new(paddr: usize, rank: usize) -> Self {
		debug_assert!(rank <= MAX_RANK, "unmapped: new: invalid rank");
		Self { paddr, rank }
	}

	pub fn from_phys(paddr: usize) -> Self {
		let pfn = addr_to_pfn(paddr);

		unsafe {
			Self {
				paddr,
				rank: index_to_meta(pfn).as_ref().rank(),
			}
		}
	}

	pub fn from_normal(vaddr: NonNull<u8>) -> Self {
		let paddr = virt_to_phys(vaddr.as_ptr() as usize);
		UnMapped::from_phys(paddr)
	}

	pub fn as_phys(&self) -> usize {
		self.paddr
	}

	pub fn rank(&self) -> usize {
		self.rank
	}

	pub unsafe fn as_mapped(self) -> NonNull<[u8]> {
		let vaddr = phys_to_virt(self.paddr);
		let size = rank_to_size(self.rank);
		let slice = from_raw_parts_mut(vaddr as *mut u8, size);

		NonNull::new_unchecked(slice)
	}
}

mod address_space;
mod address_tree;
mod test;
mod virtual_allocator;

use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;
use core::slice;

use address_space::*;
use address_tree::*;
use virtual_allocator::*;

use crate::boot::MEM_INFO;
use crate::mm::page::{map_page, PageFlag};
use crate::mm::{constant::*, util::*};

pub use address_space::AddressSpace;

pub fn allocate(layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
	VMALLOC.allocate(layout)
}

pub fn deallocate(ptr: NonNull<u8>, layout: Layout) {
	unsafe { VMALLOC.deallocate(ptr, layout) };
}

pub fn lookup_size(ptr: NonNull<u8>) -> usize {
	VMALLOC.size(ptr)
}

pub fn init() {
	VMALLOC.init(addr_to_pfn(VMALLOC_OFFSET)..addr_to_pfn(KMAP_OFFSET));
}

pub fn io_allocate(pfn: usize, count: usize) -> Result<NonNull<[u8]>, AllocError> {
	// physical address does not points I/O device
	if pfn < unsafe { MEM_INFO.end_pfn } {
		return Err(AllocError);
	}

	// TODO: pfn duplicate check
	let vaddr = ADDRESS_TREE.lock().alloc(count).ok_or_else(|| AllocError)?;

	for (vaddr, paddr) in
		(0..count).map(|x| (vaddr + x * PAGE_SIZE, pfn_to_addr(pfn) + x * PAGE_SIZE))
	{
		map_page(
			vaddr,
			paddr,
			PageFlag::Present | PageFlag::Write | PageFlag::PAT | PageFlag::PCD | PageFlag::PWT,
		)?;
	}

	unsafe {
		Ok(NonNull::from(slice::from_raw_parts(
			vaddr as *mut u8,
			rank_to_size(count),
		)))
	}
}

// static mut KMAP_PT: PT = PT::new();

// pub fn kmap(paddr: usize) -> Result<NonNull<u8>, AllocError> {}

// pub fn kunmap(vaddr: usize) {}

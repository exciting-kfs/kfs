use core::{
	alloc::{AllocError, Allocator, Layout},
	ptr::NonNull,
};

use super::{
	cache_allocator::CM,
	constant::PAGE_SHIFT,
	memory_allocator::{mem_atomic::MemAtomic, mem_normal::MemNormal, util::GFP},
	meta_page::META_PAGE_TABLE,
	page_allocator::util::addr_to_pfn,
	util::virt_to_phys,
};

pub fn kmalloc(layout: Layout, flag: GFP) -> Result<NonNull<[u8]>, AllocError> {
	match flag {
		GFP::Atomic => MemAtomic.allocate(layout),
		GFP::Normal => MemNormal.allocate(layout),
	}
}

pub unsafe fn kfree(ptr: NonNull<u8>, layout: Layout, flag: GFP) {
	match flag {
		GFP::Atomic => MemAtomic.deallocate(ptr, layout),
		GFP::Normal => MemNormal.deallocate(ptr, layout),
	}
}

pub fn ksize(ptr: NonNull<u8>) -> Option<usize> {
	unsafe { CM.cache_size(ptr) }.or_else(|| {
		let addr = ptr.as_ptr() as usize;
		let pfn = addr_to_pfn(virt_to_phys(addr));
		for n in (0..=pfn).rev() {
			let meta_page = &META_PAGE_TABLE.lock()[n];
			if meta_page.inuse() {
				return Some(1 << (meta_page.rank() + PAGE_SHIFT));
			}
		}
		None
	})
}

mod tests {
	use crate::mm::page_allocator::MAX_RANK;

	use super::*;
	use core::alloc::Layout;
	use kfs_macro::ktest;

	fn check_ksize(size: usize, align: usize) {
		let layout = unsafe { Layout::from_size_align_unchecked(size, align) };
		let ptr = MemNormal.allocate(layout).unwrap();
		assert_eq!(ksize(ptr.cast()), Some(size));
		unsafe { MemNormal.deallocate(ptr.cast(), layout) };
	}

	#[ktest]
	fn ksize_blk_test() {
		check_ksize(1 << PAGE_SHIFT, 4096);
		check_ksize(1 << PAGE_SHIFT, 4096);
		check_ksize(1 << PAGE_SHIFT << MAX_RANK, 4096);
	}

	#[ktest]
	fn ksize_cache_test() {
		check_ksize(64, 64);
		check_ksize(64, 64);
		check_ksize(128, 128);
		check_ksize(256, 256);
		check_ksize(512, 512);
		check_ksize(1024, 256);
	}
}

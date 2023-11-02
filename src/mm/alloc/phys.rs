mod atomic;
mod global;
mod memory_allocator;
mod normal;

pub use atomic::Atomic;
pub use memory_allocator::PMemAlloc;
pub use normal::Normal;

use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;

use crate::mm::page;
use crate::mm::{constant::*, util::*};

use super::cache::CM;
use super::GFP;

pub fn allocate(layout: Layout, flag: GFP) -> Result<NonNull<[u8]>, AllocError> {
	match flag {
		GFP::Atomic => Atomic.allocate(layout),
		GFP::Normal => Normal.allocate(layout),
	}
}

pub unsafe fn deallocate(ptr: NonNull<u8>, layout: Layout, flag: GFP) {
	match flag {
		GFP::Atomic => Atomic.deallocate(ptr, layout),
		GFP::Normal => Normal.deallocate(ptr, layout),
	}
}

pub fn lookup_size(ptr: NonNull<u8>) -> Option<usize> {
	unsafe { CM.cache_size(ptr) }.or_else(|| {
		let addr = ptr.as_ptr() as usize;
		let pfn = addr_to_pfn(virt_to_phys(addr));
		for n in (0..=pfn).rev() {
			let meta_page = unsafe { page::index_to_meta(n).as_mut() };
			if meta_page.is_inuse() {
				return Some(1 << (meta_page.rank() + PAGE_SHIFT));
			}
		}
		None
	})
}

pub fn init() {
	Normal::init();
	Atomic::init();
}

mod tests {
	use super::*;
	use core::alloc::Layout;
	use kfs_macro::ktest;

	fn check_ksize(size: usize, align: usize) {
		let layout = unsafe { Layout::from_size_align_unchecked(size, align) };
		let ptr = Normal.allocate(layout).unwrap();
		assert_eq!(lookup_size(ptr.cast()), Some(size));
		unsafe { Normal.deallocate(ptr.cast(), layout) };
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

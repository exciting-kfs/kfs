use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;

use crate::sync::singleton::Singleton;

use super::PMemAlloc;

pub static NORMAL_ALLOC: Singleton<PMemAlloc> = Singleton::new(PMemAlloc::uninit());

#[derive(Debug)]
pub struct MemNormal;

impl MemNormal {
	pub fn init() {
		NORMAL_ALLOC.lock().init();
	}
}

unsafe impl Allocator for MemNormal {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		NORMAL_ALLOC.lock().allocate(layout)
	}
	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		NORMAL_ALLOC.lock().deallocate(ptr, layout)
	}
}

mod tests {
	use alloc::vec::Vec;
	use kfs_macro::ktest;

	use super::MemNormal;

	#[ktest]
	fn with_collection() {
		let mut v = Vec::new_in(MemNormal);
		for _ in 0..1000000 {
			v.push(1);
		}
	}
}

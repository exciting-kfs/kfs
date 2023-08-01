use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;

use crate::sync::locked::Locked;

use super::PMemAlloc;

pub static NORMAL_ALLOC: Locked<PMemAlloc> = Locked::new(PMemAlloc::uninit());

#[derive(Debug)]
pub struct Normal;

impl Normal {
	pub fn init() {
		NORMAL_ALLOC.lock().init();
	}
}

unsafe impl Allocator for Normal {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		let mut normal = NORMAL_ALLOC.lock();
		normal.allocate(layout)
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		let mut normal = NORMAL_ALLOC.lock();
		normal.deallocate(ptr, layout)
	}
}

mod tests {
	use alloc::vec::Vec;
	use kfs_macro::ktest;

	use super::Normal;

	#[ktest]
	fn with_collection() {
		let mut v = Vec::new_in(Normal);
		for _ in 0..1000000 {
			v.push(1);
		}
	}
}

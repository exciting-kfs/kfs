use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;

use crate::sync::Locked;

use super::PMemAlloc;

pub static ATOMIC_ALLOC: Locked<PMemAlloc> = Locked::new(PMemAlloc::uninit());

#[derive(Debug, Clone, Copy, Default)]
pub struct Atomic;

impl Atomic {
	pub fn init() {
		ATOMIC_ALLOC.lock().init();
	}
}

unsafe impl Allocator for Atomic {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		let mut atomic = ATOMIC_ALLOC.lock();
		atomic.allocate(layout)
	}
	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		let mut atomic = ATOMIC_ALLOC.lock();
		atomic.deallocate(ptr, layout)
	}
}

use core::alloc::{Allocator, GlobalAlloc, Layout};
use core::ptr::NonNull;

use super::MemNormal;

/// trait Allocator vs trait GlobalAlloc
///
/// Collections in std, these use [std::alloc::Global] by default that satisfies trait [core::alloc::Allocator].
/// To change [std::alloc::Global] to our custom allocator, We should use proc-macro [global_allocator].
/// proc-macro [global_allocator] requires trait [core::alloc::GlobalAlloc], not trait [core::alloc::Allocator].

#[global_allocator]
pub static G: MemGlobal = MemGlobal;

unsafe impl Sync for MemGlobal {} // FIXME ?

pub struct MemGlobal;

unsafe impl GlobalAlloc for MemGlobal {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		match MemNormal.allocate(layout) {
			Ok(p) => p.as_ptr().cast(),
			Err(_) => 0 as *mut u8,
		}
	}
	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		if ptr.is_null() {
			return;
		}

		let ptr = NonNull::new_unchecked(ptr);
		MemNormal.deallocate(ptr, layout)
	}
}

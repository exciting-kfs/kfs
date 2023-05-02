use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::cell::UnsafeCell;
use core::ptr::NonNull;

use alloc::alloc::alloc;
use alloc::alloc::dealloc;

use crate::kmem_cache_register;

use super::slub::SizeCacheTrait;
use super::slub::SizeCache;
use super::slub::alloc_block_from_page_alloc;
use super::slub::dealloc_block_to_page_alloc;
use super::util::bit_scan_reverse;

static mut SIZE64: SizeCache<'static, 64> = SizeCache::new();		// LEVEL 6
static mut SIZE128: SizeCache<'static, 128> = SizeCache::new();
static mut SIZE256: SizeCache<'static, 256> = SizeCache::new();
static mut SIZE512: SizeCache<'static, 512> = SizeCache::new();
static mut SIZE1024: SizeCache<'static, 1024> = SizeCache::new();
static mut SIZE2048: SizeCache<'static, 2048> = SizeCache::new();	// LEVEL 11

const LEVEL_MIN: usize = 6;
const LEVEL_END: usize = 12;

/// trait Allocator vs trait GlobalAlloc
///
/// Collections in std, these use [std::alloc::Global] by default that satisfies trait [core::alloc::Allocator].
/// To change [std::alloc::Global] to our custom allocator, We should use proc-macro [global_allocator].
/// proc-macro [global_allocator] requires trait [core::alloc::GlobalAlloc], not trait [core::alloc::Allocator].

#[global_allocator]
pub static G: GlobalAllocator = GlobalAllocator::new();

pub struct GlobalAllocator(UnsafeCell<bool>); // TODO Atomic?

unsafe impl Sync for GlobalAllocator {} // ?

impl GlobalAllocator {
	pub const fn new() -> Self {
		GlobalAllocator(UnsafeCell::new(false))
	}

	pub unsafe fn lazy_init(&self) {
		if ! *self.0.get() {
			kmem_cache_register!(SIZE2048);
			kmem_cache_register!(SIZE1024);
			kmem_cache_register!(SIZE512);
			kmem_cache_register!(SIZE256);
			kmem_cache_register!(SIZE128);
			kmem_cache_register!(SIZE64);
			(*self.0.get()) = true;
		}
	}
}

unsafe impl GlobalAlloc for GlobalAllocator {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		self.lazy_init();

		let level = level_of(layout);
		match level.checked_sub(LEVEL_END) {
			None => get_allocator(level).allocate(),
			Some(rank) => match alloc_block_from_page_alloc(rank) {
				Ok(ptr) => ptr.as_mut_ptr(),
				Err(_) => 0 as *mut u8
			}
		}
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		self.lazy_init();

		if ptr.is_null() {
			return; // TODO seg-fault?
		}

		let ptr = NonNull::new_unchecked(ptr);
		let level = level_of(layout);
		match level.checked_sub(LEVEL_END) {
			None => get_allocator(level).deallocate(ptr.as_ptr()),
			Some(rank) => dealloc_block_to_page_alloc(ptr, 1, rank),
		}
	}
}

unsafe fn get_allocator<'a>(level: usize) -> &'a mut dyn SizeCacheTrait {
	let caches: [&mut dyn SizeCacheTrait; 6] = [
		&mut SIZE64,
		&mut SIZE128,
		&mut SIZE256,
		&mut SIZE512,
		&mut SIZE1024,
		&mut SIZE2048
	];
	caches[level - LEVEL_MIN]
}

fn level_of(layout: Layout) -> usize {
	let size = layout.size();
	let align = layout.align();

	if size <= 1 && align == 1 {
		return LEVEL_MIN;
	}

	let rank = unsafe { match size > align {
		true => bit_scan_reverse(size - 1) + 1,
		false => bit_scan_reverse(align - 1) + 1,
	}};

	LEVEL_MIN + rank.checked_sub(LEVEL_MIN).unwrap_or_default()
}


pub unsafe fn kmalloc(bytes: usize) -> *mut u8 {
	unsafe {
		let layout = Layout::from_size_align_unchecked(bytes, core::mem::align_of::<u8>());
		alloc(layout)
	}
}

pub unsafe fn kfree(ptr: *mut u8, len: usize) {
	let layout = Layout::from_size_align_unchecked(len, core::mem::align_of::<u8>());
	dealloc(ptr, layout)
}

mod test {

	use kfs_macro::ktest;

	#[ktest]
	fn test_alloc() {
	}

	#[ktest]
	fn test_kmalloc() {
	}
}

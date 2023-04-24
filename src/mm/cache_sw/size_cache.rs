pub mod free_list;

use core::marker::PhantomData;
use core::slice;
use core::alloc::AllocError;

use crate::mm::cache_sw::{alloc_pages_from_buddy};
use crate::pr_info;
use super::cache::{align_with_hw_cache, CacheBase, CacheShrink, CacheInit};
use super::PAGE_SIZE;

use self::free_list::{FreeList, FreeNode};


type Result<T> = core::result::Result<T, AllocError>;

#[derive(Debug)]
pub struct SizeCache<'page, const N: usize> {
	free_list: FreeList,
	page_count: usize,
	phantom: PhantomData<&'page usize>
}

impl<'page, const N: usize> SizeCache<'page, N> {
	const SIZE : usize =  align_with_hw_cache(N);

	pub const fn new() -> Self {
		SizeCache { free_list: FreeList::new(), page_count: 0, phantom: PhantomData }
	}

	pub fn alloc(&mut self) -> Result<*mut u8> {
		self.free_list.remove_if(|n| n.bytes() >= Self::SIZE)
			.or_else(|| {
				let page = self.alloc_pages(Self::SIZE / PAGE_SIZE + 1).ok()?;
				let node = unsafe { FreeNode::construct_at(page) };
				Some(node)
			}).map(|node| {
				unsafe { node.alloc_bytes(Self::SIZE) }.unwrap().map(|remains| self.free_list.insert(remains));
				node.as_mut_ptr().cast()
		}).ok_or(AllocError)
	}

	pub unsafe fn dealloc(&mut self, ptr: *mut u8) {
		if self.free_list.check_double_free(ptr) {
			panic!("size_cache: double free");
		}
		
		let ptr = slice::from_raw_parts_mut::<u8>(ptr.cast(), Self::SIZE);
		let node = FreeNode::construct_at(ptr);
		self.free_list.insert(node);
	}

	fn alloc_pages(&mut self, count: usize) -> Result<&'page mut [u8]> {
		let page = alloc_pages_from_buddy::<'page>(count).ok_or(AllocError)?;
		self.page_count += count;
		Ok(page)
	}

	// For test
	pub fn print_statistics(&self) {
		pr_info!("\npage_count: {}", self.page_count);
		pr_info!("free_node : {:?}", self.free_list);
	}
}

impl<'page, const N : usize> CacheBase for SizeCache<'_, N> {
	fn free_list(&mut self) -> &mut FreeList {
		&mut self.free_list
	}

	fn inuse(&self) -> usize {
		self.page_count
	}
}

impl<'page, const N : usize> Default for SizeCache<'_, N> {
	fn default() -> Self {
		Self::new()
	}
}

impl<'page, const N : usize> CacheShrink for SizeCache<'_, N> {}
impl<'page, const N : usize> CacheInit for SizeCache<'_, N> {}

pub trait ForSizeCache {
	fn allocate(&mut self) -> *mut u8;
	unsafe fn deallocate(&mut self, ptr: *mut u8);
}

impl<'page, const N: usize> ForSizeCache for SizeCache<'page, N> {
	fn allocate(&mut self) -> *mut u8 {
	    match self.alloc() {
		Ok(ptr) => ptr,
		Err(_) => 0 as *mut u8
	    }
	}

	unsafe fn deallocate(&mut self, ptr: *mut u8) {
	    self.dealloc(ptr);
	}
}

mod tests {
	use kfs_macro::ktest;

	use super:: {
		CacheShrink,
		SizeCache,
		PAGE_SIZE,
	};

	#[ktest]
	fn test_size() {
		const ACTUAL: [usize; 8] = [16, 16, 32, 32, 64, 64, 128, 192];

		assert_eq!(SizeCache::<0>::SIZE, ACTUAL[0]);
		assert_eq!(SizeCache::<16>::SIZE, ACTUAL[1]);
		assert_eq!(SizeCache::<17>::SIZE, ACTUAL[2]);
		assert_eq!(SizeCache::<32>::SIZE, ACTUAL[3]);
		assert_eq!(SizeCache::<33>::SIZE, ACTUAL[4]);
		assert_eq!(SizeCache::<64>::SIZE, ACTUAL[5]);
		assert_eq!(SizeCache::<65>::SIZE, ACTUAL[6]);
		assert_eq!(SizeCache::<129>::SIZE, ACTUAL[7]);
	}

	#[ktest]
	fn test_alloc() {
		const SIZE: usize = 60;
		let mut cache = SizeCache::<SIZE>::new();

		for i in 1..64 { // 64 * 63 = 4032
			let ptr = cache.alloc();
			let head_ptr = unsafe { ptr.unwrap().offset(SizeCache::<SIZE>::SIZE as isize) };
			let head = unsafe { cache.free_list.head().unwrap().as_mut() };
			assert_eq!(cache.page_count, 1);
			assert_eq!(cache.free_list.count(), 1);
			assert_eq!(head.as_ptr(), head_ptr.cast());
			assert_eq!(head.bytes(), PAGE_SIZE - SizeCache::<SIZE>::SIZE * i);
		}

		let _ = cache.alloc();
		let ptr = cache.alloc();
		let head_ptr = unsafe { ptr.unwrap().offset(SizeCache::<SIZE>::SIZE as isize) };
		let head = unsafe { cache.free_list.head().unwrap().as_mut() };
		assert_eq!(cache.page_count, 2);
		assert_eq!(cache.free_list.count(), 1);
		assert_eq!(head.as_ptr(), head_ptr.cast());
		assert_eq!(head.bytes(), PAGE_SIZE - SizeCache::<SIZE>::SIZE * 1);
	}

	#[ktest]
	fn test_dealloc() {
		fn do_test(cache: &mut SizeCache<60>, ptr: *mut u8, free_node_count: usize) {
			unsafe { cache.dealloc(ptr) };
			assert_eq!(cache.free_list.count(), free_node_count);
		}

		let mut cache = SizeCache::<60>::new();

		let ptr1 = cache.alloc().unwrap();
		let ptr2 = cache.alloc().unwrap();
		let ptr3 = cache.alloc().unwrap();
		let ptr4 = cache.alloc().unwrap();
		let ptr5 = cache.alloc().unwrap();

		do_test(&mut cache, ptr2, 2);
		do_test(&mut cache, ptr4, 3);
		do_test(&mut cache, ptr1, 3);
		do_test(&mut cache, ptr3, 2);
		do_test(&mut cache, ptr5, 1);
	}

	// #[ktest]
	// #[should_panic]
	// fn test_dealloc_double_free() {
	// 	let mut cache = SizeCache::<32>::new();
	// 	let ptr1 = cache.alloc().unwrap();
	// 	unsafe { cache.dealloc(ptr1) };
	// 	unsafe { cache.dealloc(ptr1) };
	// }


	#[ktest]
	fn test_shrink() {
		// #1 page aligned, no extra space
		let mut cache = SizeCache::<1024>::new();
		let ptr = cache.alloc().unwrap();
		unsafe {cache.dealloc(ptr)}
		cache.cache_shrink();
		assert_eq!(cache.free_list.count(), 0);

		// #2 page aligned, extra space
		let mut cache = SizeCache::<2048>::new();
		let ptr1 = cache.alloc().unwrap();
		let ptr2 = cache.alloc().unwrap();
		let ptr3 = cache.alloc().unwrap();
		cache.alloc().unwrap();

		unsafe {cache.dealloc(ptr1)}
		unsafe {cache.dealloc(ptr2)}
		unsafe {cache.dealloc(ptr3)}
		cache.cache_shrink();
		assert_eq!(cache.free_list.count(), 1);

		// #3 not aligned, no extra space
		let mut cache = SizeCache::<2048>::new();
		cache.alloc().unwrap();
		let ptr1 = cache.alloc().unwrap();
		let ptr2 = cache.alloc().unwrap();
		
		unsafe {cache.dealloc(ptr1)}
		unsafe {cache.dealloc(ptr2)}
		cache.cache_shrink();
		assert_eq!(cache.free_list.count(), 1);

		// #4 not aligned, extra space
		let mut cache = SizeCache::<2048>::new();
		cache.alloc().unwrap();
		let ptr1 = cache.alloc().unwrap();
		let ptr2 = cache.alloc().unwrap();
		let ptr3 = cache.alloc().unwrap();
		let ptr4 = cache.alloc().unwrap();
		cache.alloc().unwrap();

		unsafe {cache.dealloc(ptr1)}
		unsafe {cache.dealloc(ptr2)}
		unsafe {cache.dealloc(ptr3)}
		unsafe {cache.dealloc(ptr4)}
		cache.cache_shrink();
		assert_eq!(cache.free_list.count(), 2);
	}
}

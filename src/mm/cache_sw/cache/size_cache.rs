use core::marker::PhantomData;
use core::slice;

use crate::pr_info;
use super::utils::free_node::FreeNode;
use super::utils::free_list::*;
use super::PAGE_SIZE;

use super::{CacheShrink, CacheBase, PageAlloc, CacheInit};
use super::utils::{Error, align_with_hw_cache};

type Result<T> = core::result::Result<T, Error>;

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
				unsafe { node.alloc_bytes(Self::SIZE) }.map(|remains| self.free_list.insert(remains));
				node.as_mut_ptr().cast()
		}).ok_or(Error::Alloc)
	}

	pub unsafe fn dealloc(&mut self, ptr: *mut u8) {
		if self.free_list.check_double_free(ptr) {
			panic!("size_cache: double free");
		}
		
		let ptr = slice::from_raw_parts_mut::<u8>(ptr.cast(), Self::SIZE);
		let node = FreeNode::construct_at(ptr);
		self.free_list.insert(node);
	}

	pub fn print_statistics(&self) {
		pr_info!("\npage_count: {}", self.page_count);
		pr_info!("free_node : {}", self.free_list.count());

		self.free_list.iter().for_each(|n| {
			pr_info!("{:?}", n);
		})
	}
}

impl<'page, const N : usize> CacheBase for SizeCache<'_, N> {
	fn free_list(&mut self) -> &mut FreeList {
		&mut self.free_list
	}

	fn page_count(&mut self) -> &mut usize {
		&mut self.page_count
	}
}

impl<'page, const N : usize> Default for SizeCache<'_, N> {
	fn default() -> Self {
		Self::new()
	}
}

impl<'page, const N : usize> CacheShrink for SizeCache<'_, N> {}
impl<'page, const N : usize> PageAlloc<'page> for SizeCache<'_, N> {}
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
	use kfs_macro::kernel_test;

	use super::{super::utils::free_node::FreeNode, CacheShrink, PAGE_SIZE};

	use super::SizeCache;

	#[kernel_test(cache_size)]
	fn test_alloc() {
		const SIZE: usize = 16;
		let mut cache = SizeCache::<SIZE>::new();

		for i in 1..10 {
			let ptr = cache.alloc();
			let head_ptr = unsafe { ptr.unwrap().offset(SizeCache::<SIZE>::SIZE as isize) };
			let head = FreeNode::from_non_null(cache.free_list.head().unwrap());
			assert_eq!(cache.page_count, 1);
			assert_eq!(cache.free_list.count(), 1);
			assert_eq!(head.as_ptr(), head_ptr.cast());
			assert_eq!(head.bytes(), PAGE_SIZE - SizeCache::<SIZE>::SIZE * i);
		}
	}

	fn test_dealloc_do_test(cache: &mut SizeCache<32>, ptr: *mut u8, free_node_count: usize) {
		unsafe { cache.dealloc(ptr) };
		assert_eq!(cache.free_list.count(), free_node_count);			
	}

	#[kernel_test(cache_size)]
	fn test_dealloc() {
		let mut cache = SizeCache::<32>::new();

		let ptr1 = cache.alloc().unwrap();
		let ptr2 = cache.alloc().unwrap();
		let ptr3 = cache.alloc().unwrap();
		let ptr4 = cache.alloc().unwrap();
		let ptr5 = cache.alloc().unwrap();

		test_dealloc_do_test(&mut cache, ptr2, 2);
		test_dealloc_do_test(&mut cache, ptr4, 3);
		test_dealloc_do_test(&mut cache, ptr1, 3);
		test_dealloc_do_test(&mut cache, ptr3, 2);
		test_dealloc_do_test(&mut cache, ptr5, 1);
	}

	// #[kernel_test(cache_size)]
	// #[should_panic]
	// fn test_dealloc_double_free() {
	// 	let mut cache = SizeCache::<32>::new();
	// 	let ptr1 = cache.alloc().unwrap();
	// 	unsafe { cache.dealloc(ptr1) };
	// 	unsafe { cache.dealloc(ptr1) };
	// }


	#[kernel_test(cache_size)]
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

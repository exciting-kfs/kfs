use core::marker::PhantomData;
use core::slice;

use super::{CacheShrink, CacheBase, PageAlloc, CacheInit, PAGE_SIZE};
use super::utils::{
	self,
	free_list::*,
	free_node::FreeNode,
	Error
};

type Result<T> = core::result::Result<T, Error>;

pub struct ObjCache<'page, T> {
	free_list: FreeList,
	page_count: usize,
	phantom: PhantomData<&'page T>
}

impl<'page, T> ObjCache<'page, T> {
	const OBJ_SIZE: usize = core::mem::size_of::<T>();

	pub const fn new() -> Self {
		ObjCache { free_list: FreeList::new(), page_count: 0, phantom: PhantomData }
	}

	pub fn alloc(&mut self, count: usize) -> Result<*mut T>
	where T: CacheInit
	{
		let bytes = utils::align_with_hw_cache(Self::OBJ_SIZE * count);
		self.free_list.remove_if(|n| n.bytes() >= bytes)
			.or_else(|| {
				let extra = (bytes % PAGE_SIZE > 0) as usize;
				let count = bytes / PAGE_SIZE + extra;
				let page = self.alloc_pages(count).ok()?;
				let node = unsafe { FreeNode::construct_at(page) };
				Some(node)
			}).map(|node| {
				unsafe { node.alloc_bytes(bytes) }.map(|next| self.free_list.insert(next));
				obj_initialize(node.as_mut_ptr().cast(), count)
		}).ok_or(Error::Alloc)
	}

	pub unsafe fn dealloc(&mut self, ptr: *mut T, count: usize)
	where T: CacheInit
	{
		if self.free_list.check_double_free(ptr) {
			panic!("obj_cache: double free");
		}

		let bytes = utils::align_with_hw_cache(Self::OBJ_SIZE * count);
		let ptr = slice::from_raw_parts_mut::<u8>(ptr.cast(), bytes);
		let node = FreeNode::construct_at(ptr);
		self.free_list.insert(node);
	}
}

fn obj_initialize<T>(ptr: *mut T, count: usize) -> *mut T
where T: CacheInit
{
	for i in 0..count {
		unsafe {
			let p =  ptr.offset(i as isize);
			T::cache_init(p);
		}
	}
	ptr
}

impl<'page, T> CacheBase for ObjCache<'_, T> {
	fn free_list(&mut self) -> &mut FreeList {
	    &mut self.free_list
	}

	fn page_count(&mut self) -> &mut usize {
	    &mut self.page_count
	}
}

impl<'page, T> Default for ObjCache<'_, T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<'page, T> CacheShrink for ObjCache<'_, T> {}
impl<'page, T> PageAlloc<'page> for ObjCache<'_, T> {}
impl<'page, T> CacheInit for ObjCache<'_, T> {}

mod tests {
	use kfs_macro::kernel_test;
	use super::super::{utils::free_node::FreeNode, PAGE_SIZE};
	use super::super::{CacheShrink, CacheInit};
	use super::*;

	impl CacheInit for usize {}

	#[kernel_test(cache_obj)]
	fn test_alloc() {
		let obj_size = ObjCache::<usize>::OBJ_SIZE;
		let count = 22;
		let mut cache = ObjCache::<usize>::new();

		for i in 1..10 { // { 22 * 4(usize) = 88 -> 128 } * 9 = 1152 bytes
			let ptr = cache.alloc(count);
			let off = utils::align_with_hw_cache(count * obj_size) as isize;
			let head_ptr = unsafe { ptr.unwrap().offset(off / obj_size as isize) };
			assert_eq!(cache.page_count, 1);
			assert_eq!(cache.free_list.count(), 1);

			let head = FreeNode::from_non_null(cache.free_list.head().unwrap());
			assert_eq!(head.as_ptr(), head_ptr.cast());
			assert_eq!(head.bytes(), PAGE_SIZE - off as usize * i);
		}

		let count = 1000; // 1000 * 4(usize) = 4000 -> 4032 bytes
		cache.alloc(count).unwrap();
		assert_eq!(cache.page_count, 2);
		assert_eq!(cache.free_list.count(), 2);

		let count = 2000; // 2000 * 4(usize) = 8000 bytes >= 1page.
		cache.alloc(count).unwrap();
		assert_eq!(cache.page_count, 4);
		assert_eq!(cache.free_list.count(), 3);
	}


	fn test_dealloc_do_test(cache: &mut ObjCache<usize>, ptr: *mut usize, count: usize, free_node_count: usize) {
		unsafe { cache.dealloc(ptr, count) };
		assert_eq!(cache.free_list.count(), free_node_count);
	}

	#[kernel_test(cache_obj)]
	fn test_dealloc() {
		let mut cache = ObjCache::<usize>::new();
		let count = 200;
		let mut merged = 0;

		let ptr1 = cache.alloc(count).unwrap();	// 832
		let ptr2 = cache.alloc(count).unwrap();	// 832
		let ptr3 = cache.alloc(count).unwrap();	// 832
		let ptr4 = cache.alloc(count).unwrap();	// 832
		let ptr5 = cache.alloc(count).unwrap();	// 832
		let ptr6 = cache.alloc(count).unwrap();	// 832

		assert_eq!(cache.page_count, 2);
		assert_eq!(cache.free_list.count(), 2);

		let off = (PAGE_SIZE / core::mem::size_of::<usize>()) as isize;
		if unsafe { ptr1.offset(off) } == ptr5 {
			merged = 1;
		}

		test_dealloc_do_test(&mut cache, ptr1, count, 3);
		test_dealloc_do_test(&mut cache, ptr3, count, 4);
		test_dealloc_do_test(&mut cache, ptr5, count, 5 - merged);
		test_dealloc_do_test(&mut cache, ptr6, count, 4 - merged);
		test_dealloc_do_test(&mut cache, ptr4, count, 3 - merged);
		test_dealloc_do_test(&mut cache, ptr2, count, 2 - merged);
	}

	// #[kernel_test(cache_obj)]
	// #[should_panic]
	// fn test_dealloc_double_free() {
	// 	let mut cache = ObjCache::<usize>::new();
	// 	let ptr1 =  cache.alloc(22).unwrap();
	// 	unsafe { cache.dealloc(ptr1, 22)};
	// 	unsafe { cache.dealloc(ptr1, 22)};
	// }


	#[kernel_test(cache_obj)]
	fn test_shrink() {
		// #1 page aligned, no extra space
		let mut cache = ObjCache::<usize>::new();
		let ptr = cache.alloc(1024).unwrap();
		unsafe {cache.dealloc(ptr, 1024)}
		cache.cache_shrink();
		assert_eq!(cache.free_list.count(), 0);

		// #2 page aligned, extra space
		let mut cache = ObjCache::<usize>::new();
		let ptr = cache.alloc(1600).unwrap();
		cache.alloc(3).unwrap();
		unsafe {cache.dealloc(ptr, 1600)}
		cache.cache_shrink();
		assert_eq!(cache.free_list.count(), 2);

		// #3 not aligned, no extra space
		let mut cache = ObjCache::<usize>::new();
		let ptr = cache.alloc(1600).unwrap();
		unsafe {cache.dealloc(ptr, 1600)}
		cache.alloc(3).unwrap();
		cache.cache_shrink();
		assert_eq!(cache.free_list.count(), 1);

		// #4 not aligned, extra space
		let mut cache = ObjCache::<usize>::new();
		let ptr = cache.alloc(3000).unwrap();
		cache.alloc(3).unwrap();
		unsafe {cache.dealloc(ptr, 3000)}
		cache.alloc(3).unwrap();
		cache.cache_shrink();
		assert_eq!(cache.free_list.count(), 3);
	}
}

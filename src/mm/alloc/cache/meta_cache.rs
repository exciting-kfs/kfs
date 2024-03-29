use core::alloc::AllocError;
use core::mem::size_of;
use core::ptr::NonNull;

use super::no_alloc_list::{NAList, Node};

use crate::mm::{page::ptr_to_meta, util::*};

#[derive(Debug)]
pub struct Dummy;

#[derive(Debug)]
pub struct MetaCache {
	inuse: usize,
	pub total: usize,
	pub cache_size: usize,
	pub free_list: NAList<Dummy>,
}

impl MetaCache {
	pub const NODE_SIZE: usize = size_of::<Node<MetaCache>>();
	pub const META_SIZE: usize = align_with_hw_cache(Self::NODE_SIZE);

	/// # Safety
	///
	/// * `mem` must point memory block allocated by PAGE_ALLOC
	/// * `cache_size` must be considered the align of L1 cache.
	pub unsafe fn construct_at<'a>(mem: NonNull<u8>, cache_size: usize) -> &'a mut Self {
		let rank = get_rank(mem);
		let count = count_total(rank, Self::META_SIZE, cache_size);

		let p = mem.as_ptr() as usize + Self::META_SIZE;
		let first = next_align(p, cache_size) as *mut u8;

		let mut free_list = NAList::new();

		for i in 0..count {
			let np = first.offset((cache_size * i) as isize); // TODO overflow?
			let np = NonNull::new_unchecked(np);
			let node = Node::alloc_at(np);
			free_list.push_front(node);
		}

		let ptr = mem.as_ptr().cast::<MetaCache>();

		(*ptr).inuse = 0;
		(*ptr).total = count;
		(*ptr).cache_size = cache_size;
		(*ptr).free_list = free_list;

		&mut (*ptr)
	}

	#[inline]
	pub fn is_full(&self) -> bool {
		self.inuse == self.total
	}

	#[inline]
	pub fn is_free(&self) -> bool {
		self.inuse == 0
	}

	#[inline]
	pub fn inuse(&self) -> usize {
		self.inuse
	}

	#[inline]
	pub fn total(&self) -> usize {
		self.total
	}

	pub fn alloc(&mut self) -> Result<NonNull<[u8]>, AllocError> {
		let ptr = self
			.free_list
			.pop_front()
			.ok_or(AllocError)?
			.as_ptr()
			.cast::<u8>();

		self.inuse += 1;

		let ptr = unsafe { core::slice::from_raw_parts_mut(ptr, self.cache_size) };
		Ok(unsafe { NonNull::new_unchecked(ptr) })
	}

	/// # Safety
	/// `ptr` must point a memory block allocated by `self`
	pub unsafe fn dealloc(&mut self, ptr: NonNull<u8>) {
		self.inuse -= 1;

		let node = Node::alloc_at(ptr);
		#[cfg(ktest)]
		self.double_free_check(ptr);
		self.free_list.push_front(node);
	}

	pub fn contains(&self, ptr: NonNull<u8>) -> bool {
		let rank = self.rank();
		let size = size_of_rank(rank);
		let s = self as *const Self as usize;
		let p = ptr.as_ptr() as usize;
		match s.checked_add(size) {
			Some(e) => s <= p && p < e,
			None => s <= p && p <= usize::MAX,
		}
	}

	pub fn rank(&self) -> usize {
		get_rank(NonNull::from(self).cast())
	}

	#[cfg(ktest)]
	fn double_free_check(&mut self, ptr: NonNull<u8>) {
		self.free_list
			.find(|n| {
				let node_ptr = (*n) as *const Dummy as *const u8;
				node_ptr == ptr.as_ptr()
			})
			.map(|_| panic!("meta_cache: double free!"));
	}
}

pub fn get_rank(ptr: NonNull<u8>) -> usize {
	unsafe { ptr_to_meta(ptr).as_ref().rank() }
}

#[inline]
fn count_total(rank: usize, meta_size: usize, cache_size: usize) -> usize {
	(size_of_rank(rank) - meta_size) / cache_size
}

#[cfg(ktest)]
mod tests {
	// use super::*;
	// use core::ptr::NonNull;

	// use kfs_macro::ktest;

	#[repr(align(4096))]
	struct TestPage([u8; 4096]);

	// static mut page: TestPage = TestPage([0; 4096]);

	// #[ktest]
	// fn test_double_free_check() {
	// 	let page_ptr = unsafe { NonNull::new_unchecked(page.0.as_mut_ptr()) };
	// 	let m = unsafe { MetaCache::construct_at(page_ptr, 64) };
	// 	let ptr = unsafe { NonNull::new_unchecked(page_ptr.as_ptr().offset(32)) };
	// 	m.double_free_check(ptr);
	// }
}

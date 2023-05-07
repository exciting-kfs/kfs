use core::{mem::size_of, ptr::NonNull};

use crate::mm::{
	cache_allocator::util::{
		align_with_hw_cache,
		no_alloc_list::{NAList, Node},
	},
	meta_page::META_PAGE_TABLE,
	page_allocator::util::addr_to_pfn,
	util::{size_of_rank, virt_to_phys},
};

#[derive(Debug)]
pub struct Dummy;

#[derive(Debug)]
pub struct MetaCache {
	pub inuse: usize,
	pub cache_size: usize,
	pub free_list: NAList<Dummy>,
}

impl MetaCache {
	pub const NODE_SIZE: usize = size_of::<Node<MetaCache>>();
	pub const NODE_ALIGN: usize = align_with_hw_cache(Self::NODE_SIZE);

	/// # Safety
	///
	/// * `mem` must point memory block allocated by PAGE_ALLOC
	/// * `cache_size` must be considered the align of L1 cache.
	pub unsafe fn construct_at<'a>(mem: NonNull<u8>, cache_size: usize) -> &'a mut Self {
		let rank = get_rank(mem.as_ptr() as usize);
		let first = mem.as_ptr().offset(Self::NODE_ALIGN as isize);
		let count = (size_of_rank(rank) - Self::NODE_ALIGN) / cache_size;
		let mut free_list = NAList::new();

		for i in 0..count {
			let np = first.offset((cache_size * i) as isize); // TODO overflow?
			let np = NonNull::new_unchecked(np);
			let node = Node::alloc_at(np);
			free_list.push_front(node);
		}

		let ptr = mem.as_ptr().cast();
		(*ptr) = MetaCache {
			inuse: 0,
			cache_size,
			free_list,
		};
		&mut (*ptr)
	}

	pub fn is_full(&self) -> bool {
		let rank = self.rank();
		let cache_size = self.cache_size;
		let max = (size_of_rank(rank) - Self::NODE_ALIGN) / cache_size;

		self.inuse == max
	}

	pub fn alloc(&mut self) -> Option<NonNull<u8>> {
		self.inuse += 1;

		let ptr = self.free_list.pop_front()?.as_ptr().cast::<u8>();
		Some(unsafe { NonNull::new_unchecked(ptr) })
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
		let rank = get_rank(self as *const Self as usize);
		let size = size_of_rank(rank);
		let s = self as *const Self as usize;
		let p = ptr.as_ptr() as usize;
		match s.checked_add(size) {
			Some(e) => s <= p && p < e,
			None => s <= p && p <= usize::MAX,
		}
	}

	pub fn rank(&self) -> usize {
		get_rank(self as *const Self as usize)
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

pub fn get_rank(addr: usize) -> usize {
	let pfn = addr_to_pfn(virt_to_phys(addr));
	(&unsafe { META_PAGE_TABLE.assume_init_ref() })[pfn].rank
}

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

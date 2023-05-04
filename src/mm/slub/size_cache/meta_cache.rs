use core::{mem::size_of, ptr::NonNull};

use crate::mm::{
	meta_page::META_PAGE_TABLE,
	page_allocator::util::addr_to_pfn,
	slub::no_alloc_list::{NAList, Node},
	util::size_of_rank,
};

#[derive(Debug)]
pub struct Dummy;

#[derive(Debug)]
pub struct MetaCache {
	pub inuse: usize,
	pub free_list: NAList<Dummy>,
}

impl MetaCache {
	const NODE_SIZE: usize = size_of::<Node<MetaCache>>();

	pub unsafe fn construct_at<'a>(mem: NonNull<u8>, cache_size: usize) -> &'a mut Self {
		let rank = get_rank(mem.as_ptr() as usize);
		let first = mem.as_ptr().offset(Self::NODE_SIZE as isize);
		let count = (size_of_rank(rank) - Self::NODE_SIZE) / cache_size;
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
			free_list,
		};
		&mut (*ptr)
	}

	pub fn alloc(&mut self) -> Option<NonNull<u8>> {
		self.inuse += 1;

		let ptr = self.free_list.pop_front()?.as_ptr().cast::<u8>();
		Some(unsafe { NonNull::new_unchecked(ptr) })
	}

	pub unsafe fn dealloc(&mut self, ptr: NonNull<u8>) {
		self.inuse -= 1;

		let node = Node::alloc_at(ptr);
		self.free_list.push_front(node);
	}

	pub fn contains(&self, ptr: NonNull<u8>) -> bool {
		let rank = get_rank(self as *const Self as usize);
		let size = size_of_rank(rank);
		let s = self as *const Self as usize;
		let p = ptr.as_ptr() as usize;
		match s.checked_add(size) {
			Some(e) => s <= p && p < e,
			None => s <= p && p < usize::MAX,
		}
	}

	pub fn rank(&self) -> usize {
		get_rank(self as *const Self as usize)
	}
}

pub fn get_rank(addr: usize) -> usize {
	let pfn = addr_to_pfn(addr);
	(&unsafe { META_PAGE_TABLE.assume_init_ref() })[pfn].rank
}

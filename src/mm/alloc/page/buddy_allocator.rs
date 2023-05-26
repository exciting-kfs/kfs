//! Another Buddy allocator implementation.
//! Welcome to the WILD.

use super::free_list::FreeList;

use crate::mm::page::{index_to_meta, meta_to_index, meta_to_ptr, ptr_to_meta, MetaPage};
use crate::mm::{constant::*, util::*};

use core::alloc::AllocError;
use core::fmt::{self, Display};

use core::ops::Range;
use core::ptr::{addr_of_mut, NonNull};
use core::slice::from_raw_parts;

pub struct BuddyAlloc {
	free_list: FreeList,
}

impl BuddyAlloc {
	pub unsafe fn construct_at(ptr: *mut BuddyAlloc, mut cover_pfn: Range<usize>) {
		let free_list = FreeList::construct_at(addr_of_mut!((*ptr).free_list));

		cover_pfn.start = next_align(cover_pfn.start, rank_to_pages(MAX_RANK));
		cover_pfn.end = cover_pfn.end & !(rank_to_pages(MAX_RANK) - 1);

		for mut entry in cover_pfn
			.step_by(rank_to_pages(MAX_RANK))
			.map(|virt_pfn| pfn_virt_to_phys(virt_pfn))
			.map(|phys_pfn| index_to_meta(phys_pfn))
		{
			entry.as_mut().set_rank(MAX_RANK);
			free_list.add(entry);
		}
	}

	pub fn alloc_pages(&mut self, req_rank: usize) -> Result<NonNull<[u8]>, AllocError> {
		for rank in req_rank..=MAX_RANK {
			if let Some(page) = self.free_list.get(rank) {
				return Ok(self.split_to_rank(page, req_rank));
			}
		}
		return Err(AllocError);
	}

	pub fn free_pages(&mut self, ptr: NonNull<u8>) {
		let mut page = ptr_to_meta(ptr);
		unsafe { page.as_mut().set_inuse(false) };

		while let Some(mut buddy) = self.get_free_buddy(page) {
			unsafe { buddy.as_mut().disjoint() };
			page = unsafe { page.as_mut().merge(buddy) };
		}

		self.free_list.add(page);
	}

	fn split_to_rank(&mut self, page: NonNull<MetaPage>, req_rank: usize) -> NonNull<[u8]> {
		let mut lpage = page;
		let mut rpage;
		while req_rank < unsafe { lpage.as_mut().rank() } {
			(lpage, rpage) = unsafe { lpage.as_mut().split() };
			self.free_list.add(rpage);
		}

		unsafe { lpage.as_mut().set_inuse(true) };

		let page = meta_to_ptr(lpage);
		unsafe { NonNull::from(from_raw_parts(page.as_ptr(), rank_to_size(req_rank))) }
	}

	fn get_free_buddy(&mut self, page: NonNull<MetaPage>) -> Option<NonNull<MetaPage>> {
		let rank = unsafe { page.as_ref().rank() };

		if rank >= MAX_RANK {
			return None;
		}

		let buddy_index = meta_to_index(page) ^ rank_to_pages(rank);
		let buddy_page = unsafe { index_to_meta(buddy_index).as_ref() };

		return (!buddy_page.inuse() && unsafe { page.as_ref().rank() } == buddy_page.rank())
			.then(|| NonNull::from(buddy_page));
	}
}

impl Display for BuddyAlloc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.free_list)
	}
}

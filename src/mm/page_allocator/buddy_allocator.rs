//! Another Buddy allocator implementation.
//! Welcome to the WILD.

use super::constant::*;
use super::free_list::FreeList;
use super::util::{addr_to_pfn, addr_to_pfn_64, pfn_to_addr, rank_to_pages};

use crate::mm::meta_page::{MetaPage, META_PAGE_TABLE};
use crate::mm::util::{next_align_64, phys_to_virt, to_phys_64, virt_to_phys};

use core::fmt::{self, Display};
use core::mem::size_of;
use core::ops::Range;
use core::ptr::{addr_of_mut, NonNull};

pub struct BuddyAllocator {
	free_list: FreeList,
}

#[repr(align(4096))]
pub struct Page;

impl BuddyAllocator {
	pub unsafe fn construct_at(ptr: *mut BuddyAllocator, mut cover_mem: Range<u64>) {
		let free_list = FreeList::construct_at(addr_of_mut!((*ptr).free_list));

		cover_mem.start = next_align_64(cover_mem.start, BLOCK_SIZE as u64);

		for mut entry in cover_mem
			.step_by(BLOCK_SIZE)
			.map(|addr| addr_to_pfn_64(to_phys_64(addr)) as usize)
			.map(|pfn| NonNull::from(&mut unsafe { META_PAGE_TABLE.assume_init_mut() }[pfn]))
		{
			entry.as_mut().rank = MAX_RANK;
			free_list.add(entry);
		}
	}

	pub fn alloc_page(&mut self, req_rank: usize) -> Result<NonNull<Page>, ()> {
		for rank in req_rank..=MAX_RANK {
			if let Some(page) = self.free_list.get(rank) {
				return Ok(self.split_to_rank(page, req_rank));
			}
		}
		return Err(());
	}

	pub fn free_page(&mut self, ptr: NonNull<Page>) {
		let mut page = self.ptr_to_metapage(ptr);
		unsafe { page.as_mut().set_inuse(false) };

		while let Some(mut buddy) = self.get_free_buddy(page) {
			unsafe { buddy.as_mut().disjoint() };
			page = unsafe { page.as_mut().merge(buddy) };
		}

		self.free_list.add(page);
	}

	fn split_to_rank(&mut self, page: NonNull<MetaPage>, req_rank: usize) -> NonNull<Page> {
		let mut lpage = page;
		let mut rpage;
		while req_rank < unsafe { lpage.as_mut().rank } {
			(lpage, rpage) = unsafe { lpage.as_mut().split() };
			self.free_list.add(rpage);
		}

		unsafe { lpage.as_mut().set_inuse(true) };
		return self.metapage_to_ptr(lpage);
	}

	fn get_free_buddy(&mut self, page: NonNull<MetaPage>) -> Option<NonNull<MetaPage>> {
		let rank = unsafe { page.as_ref().rank };

		if rank >= MAX_RANK {
			return None;
		}

		let buddy_index = self.metapage_to_index(page) ^ rank_to_pages(rank);
		let buddy_page = unsafe { self.index_to_metapage(buddy_index).as_ref() };

		return (!buddy_page.is_inuse() && unsafe { page.as_ref().rank } == buddy_page.rank)
			.then(|| NonNull::from(buddy_page));
	}

	fn metapage_to_index(&self, page: NonNull<MetaPage>) -> usize {
		(page.as_ptr() as usize - unsafe { META_PAGE_TABLE.assume_init_ref().as_ptr() } as usize)
			/ size_of::<MetaPage>()
	}

	fn index_to_metapage(&mut self, index: usize) -> NonNull<MetaPage> {
		NonNull::from(&mut unsafe { META_PAGE_TABLE.assume_init_mut() }[index])
	}

	fn metapage_to_ptr(&self, page: NonNull<MetaPage>) -> NonNull<Page> {
		let index = self.metapage_to_index(page);

		return unsafe { NonNull::new_unchecked(phys_to_virt(pfn_to_addr(index)) as *mut Page) };
	}

	fn ptr_to_metapage(&mut self, ptr: NonNull<Page>) -> NonNull<MetaPage> {
		let index = addr_to_pfn(virt_to_phys(ptr.as_ptr() as usize));

		return self.index_to_metapage(index);
	}
}

impl Display for BuddyAllocator {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.free_list)
	}
}

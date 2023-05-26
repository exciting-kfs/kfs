//! free pages in each buddy allocator.

use core::ptr::{addr_of_mut, NonNull};

use crate::mm::page::MetaPage;
use crate::mm::{constant::*, util::*};

#[repr(transparent)]
/// `self.list[k]` points available `k` rank pages.
pub struct FreeList {
	list: [MetaPage; MAX_RANK + 1],
}

impl FreeList {
	pub unsafe fn construct_at(ptr: *mut FreeList) -> &'static mut FreeList {
		let base = addr_of_mut!((*ptr).list).cast::<MetaPage>();

		for p in (0..=MAX_RANK).map(|x| base.add(x)) {
			MetaPage::construct_at(p);
		}

		return &mut *ptr;
	}

	/// Add new free page into this list.
	/// Safety
	/// - `page` must be safe to convert into reference type.
	pub fn add(&mut self, page: NonNull<MetaPage>) {
		self.list[unsafe { page.as_ref().rank() }].push(page);
	}

	/// Get `rank` ranked page (if exist)
	pub fn get(&mut self, rank: usize) -> Option<NonNull<MetaPage>> {
		match rank <= MAX_RANK {
			true => self.list[rank].pop(),
			false => None,
		}
	}
}

use core::fmt::{self, Display};

impl Display for FreeList {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut total_pages = 0;
		for (rank, head) in self.list.iter().enumerate() {
			let nodes = (&head).into_iter().count();
			writeln!(
				f,
				"[R{:02}]: {:03} Node ({} PAGE)",
				rank,
				nodes,
				rank_to_pages(rank) * nodes,
			)?;

			total_pages += rank_to_pages(rank) * nodes;
		}
		write!(f, "= TOTAL {} FREE PAGES. =", total_pages)
	}
}

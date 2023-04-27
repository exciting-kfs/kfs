//! Kernel page allocator.
//! Current implementation is Buddy-allocator.
//!
//! # Terms used in implementation.
//!
//! - node: Unit of each allocation. each node holds up to `2 ^ MAX_RANK` pages.
//!
//! - page: Continous, `PAGE_SIZE` aligned, `PAGE_SIZE` sized space in memory.
//!
//! - block: Continous `2 ^ MAX_RANK` pages. simularly it's `2 ^ MAX_RANK * PAGE_SIZE` aligned.
//!
//! # Implementation detail
//!
//! After buddy_init() call memory layout(physical) will be
//! `[ KERNEL MATAPAGES BLOCK BLOCK BLOCK ... ]`
//!
//! #
//!
//!
//!
//!  
//! To get better result in term of speed,
//! We devided big continous memory into `blocks`.
//! That way, we can limit maximum (split, merge) operation per request by `MAX_RANK`.
//! But, It also reduces maximum allocation size per request by `1 block`.

pub use buddy_allocator::Page;
pub mod util;

mod buddy_allocator;
mod constant;
mod free_list;

use self::constant::VM_OFFSET;

use super::x86::init::VMemory;
use buddy_allocator::BuddyAllocator;
use core::{
	mem::MaybeUninit,
	ptr::{addr_of_mut, NonNull},
};

pub struct PageAllocator {
	high: BuddyAllocator,
	normal: BuddyAllocator,
}

pub enum GFP {
	Normal,
	High,
}

pub static mut PAGE_ALLOC: MaybeUninit<PageAllocator> = MaybeUninit::uninit();

impl PageAllocator {
	pub unsafe fn init(vm: &VMemory) {
		BuddyAllocator::construct_at(
			addr_of_mut!((*PAGE_ALLOC.as_mut_ptr()).normal),
			vm.reserved.end..vm.normal.end,
		);

		BuddyAllocator::construct_at(
			addr_of_mut!((*PAGE_ALLOC.as_mut_ptr()).high),
			vm.high.start..vm.high.end,
		);
	}

	pub fn alloc_page(&mut self, rank: usize, flag: GFP) -> Result<NonNull<Page>, ()> {
		match flag {
			GFP::High => self.high.alloc_page(rank),
			GFP::Normal => Err(()),
		}
		.or_else(|_| self.normal.alloc_page(rank))
	}

	pub fn free_page(&mut self, page: NonNull<Page>) {
		let addr = page.as_ptr() as usize;

		match addr < VM_OFFSET {
			true => self.high.free_page(page),
			false => self.normal.free_page(page),
		};
	}
}

// #[cfg(ktest)]
mod mmtest {
	use crate::{
		collection::WrapQueue,
		mm::{
			constant::{PAGE_SHIFT, PAGE_SIZE},
			Page,
		},
	};

	use super::*;
	use crate::util::LCG;
	use kfs_macro::ktest;

	static mut PAGE_STATE: [bool; (usize::MAX >> PAGE_SHIFT) + 1] =
		[false; (usize::MAX >> PAGE_SHIFT) + 1];

	fn reset_page_state() {
		for x in unsafe { PAGE_STATE.iter_mut() } {
			*x = false;
		}
	}

	fn mark_alloced(addr: usize, rank: usize) {
		let pfn = addr >> PAGE_SHIFT;

		for i in pfn..(pfn + (1 << rank)) {
			unsafe {
				if PAGE_STATE[i] {
					panic!("allocation overwrapped!");
				}
				PAGE_STATE[i] = true;
			}
		}
	}

	fn mark_freed(addr: usize, rank: usize) {
		let pfn = addr >> PAGE_SHIFT;

		for i in pfn..(pfn + (1 << rank)) {
			unsafe {
				if !PAGE_STATE[i] {
					panic!("double free detected.");
				}
				PAGE_STATE[i] = false;
			}
		}
	}

	fn checked_alloc(rank: usize) -> Result<NonNull<Page>, ()> {
		let mem = unsafe { PAGE_ALLOC.assume_init_mut() }.alloc_page(rank, GFP::Normal)?;

		assert!(mem.as_ptr() as usize % PAGE_SIZE == 0);

		mark_alloced(mem.as_ptr() as usize, rank);

		Ok(mem)
	}

	fn checked_free(page: NonNull<Page>, rank: usize) {
		mark_freed(page.as_ptr() as usize, rank);

		unsafe { PAGE_ALLOC.assume_init_mut() }.free_page(page);
	}

	#[ktest]
	pub fn min_rank_alloc_free() {
		reset_page_state();

		// allocate untill OOM
		while let Ok(_) = checked_alloc(0) {}

		// free all
		for (i, is_alloced) in unsafe { PAGE_STATE }.iter().enumerate() {
			if *is_alloced {
				checked_free(NonNull::new((i << PAGE_SHIFT) as *mut Page).unwrap(), 0);
			}
		}
	}

	#[ktest]
	pub fn random_alloc_free() {
		reset_page_state();

		let mut queue: WrapQueue<(NonNull<Page>, usize), 100> =
			WrapQueue::from_fn(|_| (NonNull::dangling(), 0));

		let mut rng = LCG::new(42);

		while !queue.full() {
			let rank = rng.rand() as usize % (10 + 1);
			queue.push((checked_alloc(rank).unwrap(), rank));
		}

		while !queue.empty() {
			let (page, rank) = queue.pop().unwrap();
			checked_free(page, rank);
		}
	}
}

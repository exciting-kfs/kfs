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
pub use constant::MAX_RANK;

pub mod util;

mod buddy_allocator;
mod constant;
mod free_list;

use crate::util::singleton::Singleton;

use self::constant::VM_OFFSET;

use super::x86::init::VMemory;
use buddy_allocator::BuddyAllocator;
use core::ptr::{addr_of_mut, NonNull};

pub struct PageAllocator {
	high: BuddyAllocator,
	normal: BuddyAllocator,
}

pub enum GFP {
	Normal,
	High,
}

pub static PAGE_ALLOC: Singleton<PageAllocator> = Singleton::uninit();

impl PageAllocator {
	pub unsafe fn init(vm: &VMemory) {
		let ptr = PAGE_ALLOC.as_ptr();

		BuddyAllocator::construct_at(addr_of_mut!((*ptr).normal), vm.normal_pfn.clone());
		BuddyAllocator::construct_at(addr_of_mut!((*ptr).high), vm.high_pfn.clone());
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

mod mmtest {
	use crate::{
		collection::WrapQueue,
		mm::{
			constant::{PAGE_SHIFT, PAGE_SIZE},
			Page,
		},
		pr_info,
	};

	use super::{constant::MAX_RANK, *};
	use crate::util::lcg::LCG;
	use kfs_macro::ktest;

	static mut PAGE_STATE: [bool; (usize::MAX >> PAGE_SHIFT) + 1] =
		[false; (usize::MAX >> PAGE_SHIFT) + 1];

	const RANDOM_SEED: u32 = 42;
	const ALLOC_QUEUE_SIZE: usize = 100;

	type AllocQueue = WrapQueue<AllocInfo, ALLOC_QUEUE_SIZE>;

	#[derive(Clone, Copy)]
	struct AllocInfo {
		pub ptr: NonNull<Page>,
		pub rank: usize,
	}

	impl Default for AllocInfo {
		fn default() -> Self {
			Self {
				ptr: NonNull::dangling(),
				rank: 0,
			}
		}
	}

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

	fn alloc(rank: usize, flag: GFP) -> Result<AllocInfo, ()> {
		let mem = PAGE_ALLOC.lock().get_mut().alloc_page(rank, flag)?;

		assert!(mem.as_ptr() as usize % PAGE_SIZE == 0);

		mark_alloced(mem.as_ptr() as usize, rank);

		Ok(AllocInfo { ptr: mem, rank })
	}

	fn free(info: AllocInfo) {
		mark_freed(info.ptr.as_ptr() as usize, info.rank);

		PAGE_ALLOC.lock().get_mut().free_page(info.ptr);
	}

	fn is_zone_normal(ptr: NonNull<Page>) -> bool {
		let addr = ptr.as_ptr() as usize;

		return addr >= VM_OFFSET;
	}

	#[ktest]
	pub fn min_rank_alloc_free() {
		reset_page_state();

		// allocate untill OOM
		let mut count = 0;
		while let Ok(_) = alloc(0, GFP::Normal) {
			count += 1;
		}

		pr_info!(
			" note: {} page ({}MB) allocated from ZONE_NORMAL",
			count,
			count * PAGE_SIZE / 1024 / 1024
		);

		// free all
		for (i, is_alloced) in unsafe { PAGE_STATE }.iter().enumerate() {
			if *is_alloced {
				free(AllocInfo {
					ptr: NonNull::new((i << PAGE_SHIFT) as *mut Page).unwrap(),
					rank: 0,
				});
			}
		}
	}

	#[ktest]
	pub fn random_alloc_free() {
		reset_page_state();

		let mut queue = AllocQueue::with(Default::default());

		let mut rng = LCG::new(RANDOM_SEED);

		while !queue.full() {
			let rank = rng.rand() as usize % (MAX_RANK + 1);
			queue.push(alloc(rank, GFP::Normal).unwrap());
		}

		while !queue.empty() {
			free(queue.pop().unwrap());
		}
	}

	#[ktest]
	pub fn random_order_alloc_free() {
		reset_page_state();

		let mut queue = AllocQueue::with(Default::default());

		let mut rng = LCG::new(RANDOM_SEED);

		while !queue.full() {
			let rank = rng.rand() as usize % (MAX_RANK + 1);
			queue.push(alloc(rank, GFP::Normal).unwrap());
		}

		// shuffle
		for _ in 0..1000 {
			let a = rng.rand() as usize % ALLOC_QUEUE_SIZE;
			let b = rng.rand() as usize % ALLOC_QUEUE_SIZE;

			let temp = *queue.at(a).unwrap();
			*queue.at_mut(a).unwrap() = *queue.at_mut(b).unwrap();
			*queue.at_mut(b).unwrap() = temp;
		}

		while !queue.empty() {
			free(queue.pop().unwrap());
		}
	}

	#[ktest]
	pub fn zone_high_basic() {
		let info = alloc(0, GFP::High).unwrap();

		assert!(!is_zone_normal(info.ptr));

		free(info);
	}

	#[ktest]
	pub fn zone_high() {
		reset_page_state();

		let mut count = 0;
		loop {
			let info = alloc(0, GFP::High).expect("OOM before ZONE_NORMAL is exhausted");

			if is_zone_normal(info.ptr) {
				break;
			}

			count += 1;
		}

		pr_info!(
			" note: {} page ({}MB) allocated from ZONE_HIGH",
			count,
			count * PAGE_SIZE / 1024 / 1024
		);

		for (i, is_alloced) in unsafe { PAGE_STATE }.iter().enumerate() {
			if *is_alloced {
				free(AllocInfo {
					ptr: NonNull::new((i << PAGE_SHIFT) as *mut Page).unwrap(),
					rank: 0,
				});
			}
		}
	}
}

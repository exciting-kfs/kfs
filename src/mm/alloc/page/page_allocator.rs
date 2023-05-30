//! Kernel page allocator.

use super::buddy_allocator::BuddyAlloc;

use crate::mm::alloc::Zone;
use crate::mm::constant::*;
use crate::mm::page::VMemory;
use crate::sync::singleton::Singleton;

use core::alloc::AllocError;
use core::ptr::{addr_of_mut, NonNull};

/// PageAlloc Holds 3 different buddy allocator.
/// - high, vmalloc: allocate from ZONE_HIGH (not mapped)
/// - normal: allocate from ZONE_NORMAL (linear mapped)
pub struct PageAlloc {
	high: BuddyAlloc,
	vmalloc: BuddyAlloc,
	normal: BuddyAlloc,
}

pub static PAGE_ALLOC: Singleton<PageAlloc> = Singleton::uninit();

impl PageAlloc {
	pub unsafe fn init(vm: &VMemory) {
		let ptr = unsafe { PAGE_ALLOC.as_mut_ptr() };

		BuddyAlloc::construct_at(addr_of_mut!((*ptr).normal), vm.normal_pfn.clone());
		BuddyAlloc::construct_at(addr_of_mut!((*ptr).high), vm.high_pfn.clone());
		BuddyAlloc::construct_at(addr_of_mut!((*ptr).vmalloc), vm.vmalloc_pfn.clone());
	}

	/// Allocate new `rank` ranked pages from zone `flag`.
	///
	/// Note that if requested zone is `Zone::High`
	/// and there is no sufficient pages in `Zone::High`, then
	/// pages from `Zone::Normal` can be returned.
	pub fn alloc_pages(&mut self, rank: usize, flag: Zone) -> Result<NonNull<[u8]>, AllocError> {
		match flag {
			Zone::High => self
				.high
				.alloc_pages(rank)
				.or_else(|_| self.vmalloc.alloc_pages(rank)),
			Zone::Normal => Err(AllocError),
		}
		.or_else(|_| self.normal.alloc_pages(rank))
	}

	/// Deallocate pages.
	pub fn free_pages(&mut self, page: NonNull<u8>) {
		let addr = page.as_ptr() as usize;

		if addr < VM_OFFSET {
			self.high.free_pages(page)
		} else if addr < VMALLOC_OFFSET {
			self.normal.free_pages(page)
		} else {
			self.vmalloc.free_pages(page)
		};
	}
}

mod test {
	use super::*;

	use crate::{pr_info, util::lcg::LCG};
	use kfs_macro::ktest;

	use alloc::{collections::LinkedList, vec::Vec};

	static mut PAGE_STATE: [bool; (usize::MAX >> PAGE_SHIFT) + 1] =
		[false; (usize::MAX >> PAGE_SHIFT) + 1];

	const RANDOM_SEED: u32 = 42;
	const ALLOC_QUEUE_SIZE: usize = 100;

	#[derive(Clone, Copy, Debug)]
	struct AllocInfo {
		ptr: NonNull<u8>,
		rank: usize,
	}

	impl AllocInfo {
		pub fn new(mut ptr: NonNull<[u8]>) -> Self {
			let slice = unsafe { ptr.as_mut() };

			let rank = (slice.len() / PAGE_SIZE).ilog2() as usize;
			let ptr = unsafe { NonNull::new_unchecked(slice.as_mut_ptr()) };

			Self { ptr, rank }
		}

		pub fn as_non_null(&self) -> NonNull<u8> {
			self.ptr
		}

		pub fn as_ptr(&self) -> *const u8 {
			self.ptr.as_ptr().cast_const()
		}

		pub fn rank(&self) -> usize {
			self.rank
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

	fn alloc(rank: usize, flag: Zone) -> Result<AllocInfo, AllocError> {
		let mem = AllocInfo::new(PAGE_ALLOC.lock().alloc_pages(rank, flag)?);

		assert!(mem.as_ptr() as usize % PAGE_SIZE == 0);
		assert!(mem.rank() == rank);

		mark_alloced(mem.as_ptr() as usize, rank);

		Ok(mem)
	}

	fn free(info: AllocInfo) {
		mark_freed(info.ptr.as_ptr() as usize, info.rank);

		PAGE_ALLOC.lock().free_pages(info.ptr);
	}

	fn is_zone_normal(ptr: NonNull<u8>) -> bool {
		let addr = ptr.as_ptr() as usize;

		return VM_OFFSET <= addr && addr < VMALLOC_OFFSET;
	}

	fn free_all() {
		for (i, is_alloced) in unsafe { PAGE_STATE }.iter().enumerate() {
			if *is_alloced {
				free(AllocInfo {
					ptr: NonNull::new((i << PAGE_SHIFT) as *mut u8).unwrap(),
					rank: 0,
				});
			}
		}
	}

	#[ktest]
	pub fn min_rank_alloc_free() {
		reset_page_state();

		// allocate untill OOM
		let mut count = 0;
		while let Ok(_) = alloc(0, Zone::Normal) {
			count += 1;
		}

		pr_info!(
			" note: {} page ({}MB) allocated from ZONE_NORMAL",
			count,
			count * PAGE_SIZE / 1024 / 1024
		);
		free_all();
	}

	#[ktest]
	pub fn random_alloc_free() {
		reset_page_state();

		let mut queue = LinkedList::new();

		let mut rng = LCG::new(RANDOM_SEED);

		for _ in 0..ALLOC_QUEUE_SIZE {
			let rank = rng.rand() as usize % (MAX_RANK + 1);
			queue.push_back(alloc(rank, Zone::Normal).unwrap());
		}

		while let Some(info) = queue.pop_front() {
			free(info);
		}
	}

	#[ktest]
	pub fn random_order_alloc_free() {
		reset_page_state();

		let mut queue = Vec::new();

		let mut rng = LCG::new(RANDOM_SEED);

		for _ in 0..ALLOC_QUEUE_SIZE {
			let rank = rng.rand() as usize % (MAX_RANK + 1);
			queue.push(alloc(rank, Zone::Normal).unwrap());
		}

		for _ in 0..(ALLOC_QUEUE_SIZE * 10) {
			let l = rng.rand() as usize % queue.len();
			let r = rng.rand() as usize % queue.len();

			queue.swap(l, r);
		}

		while let Some(info) = queue.pop() {
			free(info);
		}
	}

	#[ktest]
	pub fn zone_high_basic() {
		let info = alloc(0, Zone::High).unwrap();

		assert!(!is_zone_normal(info.ptr));

		free(info);
	}

	#[ktest]
	pub fn zone_high() {
		reset_page_state();

		let mut count = 0;
		loop {
			let info = alloc(0, Zone::High).expect("OOM before ZONE_NORMAL is exhausted");

			if is_zone_normal(info.ptr) {
				break;
			}

			count += 1;
		}

		pr_info!(
			" note: {} page ({}MB) allocated from ZONE_HIGH",
			count,
			count * PAGE_SIZE / MB
		);

		free_all();
	}
}

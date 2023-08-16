//! Kernel page allocator.

use super::buddy_allocator::BuddyAlloc;

use crate::boot::MEM_INFO;
use crate::mm::alloc::Zone;
use crate::mm::{constant::*, util::*};
use crate::ptr::UnMapped;
use crate::sync::locked::Locked;

use core::alloc::AllocError;
use core::mem::MaybeUninit;
use core::ptr::{addr_of_mut, NonNull};

/// PageAlloc Holds 2 different buddy allocator.
/// - high: allocate from ZONE_HIGH (not mapped)
/// - normal: allocate from ZONE_NORMAL (linear mapped)
pub struct PageAlloc {
	available_pages: usize,
	high: BuddyAlloc,
	normal: BuddyAlloc,
}

pub(super) static PAGE_ALLOC: Locked<MaybeUninit<PageAlloc>> = Locked::uninit();

impl PageAlloc {
	pub unsafe fn init() {
		let mut page_alloc = PAGE_ALLOC.lock();

		let ptr = page_alloc.as_mut_ptr();
		let mem = &mut MEM_INFO;

		BuddyAlloc::construct_at(
			addr_of_mut!((*ptr).normal),
			mem.normal_start_pfn..mem.high_start_pfn,
		);
		BuddyAlloc::construct_at(addr_of_mut!((*ptr).high), mem.high_start_pfn..mem.end_pfn);

		(*ptr).available_pages = mem.end_pfn - mem.normal_start_pfn;
	}

	/// Allocate new `rank` ranked pages from zone `flag`.
	///
	/// Note that if requested zone is `Zone::High`
	/// and there is no sufficient pages in `Zone::High`, then
	/// pages from `Zone::Normal` can be returned.
	pub fn alloc_pages(&mut self, rank: usize, flag: Zone) -> Result<UnMapped, AllocError> {
		let pages = match flag {
			Zone::High => self.high.alloc_pages(rank),
			Zone::Normal => Err(AllocError),
		}
		.or_else(|_| self.normal.alloc_pages(rank));

		if pages.is_ok() {
			self.available_pages -= rank_to_pages(rank);
		}

		pages
	}

	/// Deallocate pages.
	pub fn free_pages(&mut self, page: UnMapped) {
		let pfn = addr_to_pfn(page.as_phys());
		self.available_pages += rank_to_pages(page.rank());

		if pfn < unsafe { MEM_INFO.high_start_pfn } {
			self.normal.free_pages(page)
		} else {
			self.high.free_pages(page)
		};
	}

	pub fn get_available_pages(&self) -> usize {
		self.available_pages
	}
}

mod test {
	use super::*;

	use crate::{
		mm::alloc::page::{alloc_pages, free_pages},
		pr_info, pr_warn,
		util::lcg::LCG,
	};
	use kfs_macro::ktest;

	use alloc::{collections::LinkedList, vec::Vec};

	static mut PAGE_STATE: [bool; (usize::MAX >> PAGE_SHIFT) + 1] =
		[false; (usize::MAX >> PAGE_SHIFT) + 1];

	const RANDOM_SEED: u32 = 42;
	const ALLOC_QUEUE_SIZE: usize = 100;

	#[derive(Clone, Debug)]
	struct AllocInfo {
		ptr: UnMapped,
	}

	impl AllocInfo {
		pub fn new(ptr: UnMapped) -> Self {
			Self { ptr }
		}

		pub fn as_non_null(&self) -> NonNull<u8> {
			unsafe { self.ptr.clone().as_mapped().cast() }
		}

		pub fn rank(&self) -> usize {
			self.ptr.rank()
		}

		pub fn as_unmapped(&self) -> UnMapped {
			self.ptr.clone()
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
		let mem = AllocInfo::new(alloc_pages(rank, flag.clone())?);

		assert!(mem.ptr.as_phys() % PAGE_SIZE == 0);
		assert!(mem.rank() == rank);

		mark_alloced(mem.ptr.as_phys() as usize, rank);

		if let Zone::Normal = flag {
			unsafe { *mem.as_non_null().as_ptr() = 42 };
		}

		Ok(mem)
	}

	fn free(info: AllocInfo) {
		mark_freed(info.ptr.as_phys() as usize, info.rank());

		free_pages(info.as_unmapped());
	}

	fn is_zone_normal(ptr: NonNull<u8>) -> bool {
		let addr = ptr.as_ptr() as usize;

		return VM_OFFSET <= addr && addr < VMALLOC_OFFSET;
	}

	fn free_all() {
		for (i, is_alloced) in unsafe { &mut PAGE_STATE }.iter().enumerate() {
			if *is_alloced {
				free(AllocInfo {
					ptr: UnMapped::from_phys(pfn_to_addr(i)),
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
			(count * PAGE_SIZE) / MB
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

		if is_zone_normal(info.as_non_null()) {
			pr_warn!(
				concat!(
					" note: allocated memory came from `ZONE_NORMAL`\n",
					"  if installed memory > {} MB, then it could be BUG",
				),
				virt_to_phys(VMALLOC_OFFSET) / MB
			);
		}

		free(info);
	}

	#[ktest]
	pub fn zone_high() {
		reset_page_state();

		let mut count = 0;
		loop {
			let info = alloc(0, Zone::High).expect("OOM before ZONE_NORMAL is exhausted");

			if is_zone_normal(info.as_non_null()) {
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

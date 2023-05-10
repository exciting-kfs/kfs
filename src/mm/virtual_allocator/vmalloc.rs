use alloc::collections::BTreeSet;
use core::alloc::{AllocError, Allocator, Layout};

use core::cmp::Ordering;
use core::iter::repeat_with;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use core::ops::{
	Bound::{Excluded, Included, Unbounded},
	Range,
};
use core::slice::from_raw_parts;

use crate::mm::constant::PAGE_SIZE;
use crate::mm::meta_page::{metapage_let, MetaPage, MetaPageTable, META_PAGE_TABLE};
use crate::mm::util::virt_to_phys;
use crate::mm::x86::init::GLOBAL_PD_VIRT;
use crate::mm::x86::x86_page::{PageFlag, PD};
use crate::mm::{addr_to_pfn, pfn_to_addr, GFP, PAGE_ALLOC};
use crate::pr_err;

#[derive(PartialEq, Eq, Clone, Copy)]
struct Page {
	pub pfn: usize,
	pub count: usize,
}

impl Page {
	pub fn new(pfn: usize, count: usize) -> Self {
		Self { pfn, count }
	}

	pub fn from_pfn(pfn: usize) -> Self {
		Self { pfn, count: 0 }
	}

	pub fn from_count(count: usize) -> Self {
		Self { pfn: 0, count }
	}

	pub fn end_pfn(&self) -> usize {
		self.pfn + self.count
	}
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct OrdByPfn(pub Page);

impl From<Page> for OrdByPfn {
	fn from(value: Page) -> Self {
		Self(value)
	}
}

impl PartialOrd for OrdByPfn {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		match self.0.pfn.partial_cmp(&other.0.pfn) {
			Some(Ordering::Equal) => self.0.count.partial_cmp(&other.0.count),
			x => x,
		}
	}
}

impl Ord for OrdByPfn {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		match self.0.pfn.cmp(&other.0.pfn) {
			Ordering::Equal => self.0.count.cmp(&other.0.count),
			x => x,
		}
	}
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct OrdByCount(pub Page);

impl From<Page> for OrdByCount {
	fn from(value: Page) -> Self {
		Self(value)
	}
}

impl PartialOrd for OrdByCount {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		match self.0.count.partial_cmp(&other.0.count) {
			Some(Ordering::Equal) => self.0.pfn.partial_cmp(&other.0.pfn),
			x => x,
		}
	}
}

impl Ord for OrdByCount {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		match self.0.count.cmp(&other.0.count) {
			Ordering::Equal => self.0.pfn.cmp(&other.0.pfn),
			x => x,
		}
	}
}

struct AddressTree {
	area: Range<usize>,
	by_count: BTreeSet<OrdByCount>,
	by_pfn: BTreeSet<OrdByPfn>,
}

impl AddressTree {
	pub fn new(area: Range<usize>) -> Self {
		let mut by_count = BTreeSet::new();
		let mut by_pfn = BTreeSet::new();

		let page = Page::new(area.start, area.end - area.start);
		by_count.insert(OrdByCount(page));
		by_pfn.insert(OrdByPfn(page));

		Self {
			area,
			by_count,
			by_pfn,
		}
	}

	pub fn alloc(&mut self, count: usize) -> Option<usize> {
		let page = self
			.by_count
			.range((Included(OrdByCount(Page::from_count(count))), Unbounded))
			.next()?
			.0;

		self.by_count.remove(&OrdByCount(page));
		self.by_pfn.remove(&OrdByPfn(page));

		let new_page = Page::new(page.pfn + count, page.count - count);

		self.by_count.insert(new_page.into());
		self.by_pfn.insert(new_page.into());

		Some(pfn_to_addr(page.pfn))
	}

	pub fn dealloc(&mut self, addr: usize, count: usize) {
		let page = Page::new(addr_to_pfn(addr), count);

		let lower = self
			.by_pfn
			.range((Unbounded, Excluded(OrdByPfn(page))))
			.next_back()
			.map(|x| x.0.clone())
			.and_then(|x| {
				if x.end_pfn() == page.pfn {
					self.by_pfn.remove(&OrdByPfn(x));
					self.by_count.remove(&OrdByCount(x));

					Some(x.pfn)
				} else {
					None
				}
			})
			.unwrap_or_else(|| page.pfn);

		let upper = self
			.by_pfn
			.range((Excluded(OrdByPfn(page)), Unbounded))
			.next()
			.map(|x| x.0.clone())
			.and_then(|x| {
				if x.pfn == page.end_pfn() {
					self.by_pfn.remove(&OrdByPfn(x));
					self.by_count.remove(&OrdByCount(x));

					Some(x.end_pfn())
				} else {
					None
				}
			})
			.unwrap_or_else(|| page.end_pfn());

		let page = Page::new(lower, upper - lower);

		self.by_pfn.insert(OrdByPfn(page));
		self.by_count.insert(OrdByCount(page));
	}
}

enum VMallocError {
	OutOfAddress,
	OutOfMemory(usize),
}

static mut ADDRESS_TREE: MaybeUninit<AddressTree> = MaybeUninit::uninit();
pub struct VirtualAllocator;
pub static VMALLOC: VirtualAllocator = VirtualAllocator;

impl VirtualAllocator {
	pub fn init(&self, area: Range<usize>) {
		unsafe { ADDRESS_TREE.write(AddressTree::new(area)) };
	}

	fn addr_tree(&self) -> &mut AddressTree {
		unsafe { ADDRESS_TREE.assume_init_mut() }
	}

	fn global_pd(&self) -> &mut PD {
		unsafe { &mut GLOBAL_PD_VIRT }
	}

	fn try_allocate(
		&self,
		head: &mut MetaPage,
		pages: usize,
	) -> Result<NonNull<[u8]>, VMallocError> {
		let base_address = self
			.addr_tree()
			.alloc(pages)
			.ok_or_else(|| VMallocError::OutOfAddress)?;

		for vaddr in (0..pages).map(|x| base_address + x * PAGE_SIZE) {
			let paddr = virt_to_phys(
				PAGE_ALLOC
					.lock()
					.alloc_page(0, GFP::High)
					.map_err(|_| VMallocError::OutOfMemory(base_address))?
					.as_ptr() as usize,
			);

			head.push(NonNull::from(
				&mut META_PAGE_TABLE.lock()[addr_to_pfn(paddr)],
			));

			self.global_pd()
				.map_page(
					vaddr,
					paddr,
					PageFlag::Present | PageFlag::Global | PageFlag::Write,
				)
				.map_err(|_| VMallocError::OutOfMemory(base_address))?;
		}

		Ok(NonNull::from(unsafe {
			from_raw_parts(base_address as *const u8, pages * PAGE_SIZE)
		}))
	}

	fn layout_to_pages(layout: Layout) -> usize {
		(layout.size() / PAGE_SIZE) + 1
	}

	fn free_pages(&self, head: &mut MetaPage, base_addr: usize, pages: usize) {
		for (i, entry) in repeat_with(|| head.pop()).map_while(|v| v).enumerate() {
			let page = MetaPageTable::metapage_to_ptr(entry);
			PAGE_ALLOC.lock().free_page(page);

			let addr = base_addr + i * PAGE_SIZE;
			let _ = self.global_pd().unmap_page(addr);
		}

		self.addr_tree().dealloc(base_addr, pages);
	}
}

unsafe impl Allocator for VirtualAllocator {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		let pages = Self::layout_to_pages(layout);

		metapage_let![dummy];
		let result = self.try_allocate(dummy, pages);

		if let Err(VMallocError::OutOfMemory(mem)) = result {
			self.free_pages(dummy, mem, pages);
		}

		dummy.disjoint();

		result.map_err(|_| AllocError)
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		let virt_base_addr = ptr.as_ptr() as usize;
		let phys_base_addr = match self.global_pd().lookup(virt_base_addr) {
			Some(x) => x,
			None => {
				pr_err!(
					"VirtualAllocator: Deallocation requested with not allocated pointer({:p})",
					ptr,
				);
				return;
			}
		};

		metapage_let![dummy];

		dummy.push(NonNull::from(
			&mut META_PAGE_TABLE.lock()[addr_to_pfn(phys_base_addr)],
		));

		let pages = Self::layout_to_pages(layout);
		self.free_pages(dummy, virt_base_addr, pages)
	}
}

mod test {
	use crate::{
		mm::constant::{MB, PAGE_SHIFT, PAGE_SIZE},
		pr_info,
		util::lcg::LCG,
	};

	use super::*;
	use crate::mm::constant::PT_COVER_SIZE;
	use alloc::vec::Vec;
	use kfs_macro::ktest;

	static mut PAGE_STATE: [bool; (usize::MAX >> PAGE_SHIFT) + 1] =
		[false; (usize::MAX >> PAGE_SHIFT) + 1];

	const RANDOM_SEED: u32 = 42;
	const ALLOC_QUEUE_SIZE: usize = 100;

	#[derive(Clone, Copy)]
	struct AllocInfo {
		pub ptr: NonNull<[u8]>,
		pub layout: Layout,
	}

	fn reset_page_state() {
		for x in unsafe { PAGE_STATE.iter_mut() } {
			*x = false;
		}
	}

	fn mark_alloced(addr: usize, size: usize) {
		let pfn = addr >> PAGE_SHIFT;

		for i in pfn..(pfn + (size / PAGE_SIZE) + 1) {
			unsafe {
				if PAGE_STATE[i] {
					panic!("allocation overwrapped!");
				}
				PAGE_STATE[i] = true;
			}
		}
	}

	fn mark_freed(addr: usize, size: usize) {
		let pfn = addr >> PAGE_SHIFT;

		for i in pfn..(pfn + (size / PAGE_SIZE) + 1) {
			unsafe {
				if !PAGE_STATE[i] {
					panic!("double free detected.");
				}
				PAGE_STATE[i] = false;
			}
		}
	}

	fn alloc(size: usize) -> Result<AllocInfo, ()> {
		let layout = Layout::from_size_align(size, PAGE_SIZE).unwrap();
		let mem = VMALLOC.allocate(layout).or_else(|_| Err(()))?;

		assert!(unsafe { mem.as_ref().len() >= size });

		let addr = unsafe { mem.as_ref().as_ptr() as usize };
		assert!(addr % PAGE_SIZE == 0);

		mark_alloced(addr, size);

		unsafe { core::ptr::write_bytes(addr as *mut u8, 0, size) };

		Ok(AllocInfo { ptr: mem, layout })
	}

	fn free(info: AllocInfo) {
		let addr = unsafe { info.ptr.as_ref().as_ptr() as usize };
		mark_freed(addr, info.layout.size());
		unsafe { VMALLOC.deallocate(NonNull::new_unchecked(addr as *mut u8), info.layout) };
	}

	fn show_mapping() {
		for (i, entry) in VMALLOC.global_pd().iter().enumerate() {
			if entry.flag().contains(PageFlag::Present) {
				pr_info!("P = {:#0x} => V = {:#0x}", entry.addr(), i * PT_COVER_SIZE);
			}
		}
	}

	#[ktest]
	pub fn vmalloc_basic() {
		reset_page_state();

		let data = alloc(2 * PAGE_SIZE).unwrap();

		free(data);
	}

	#[ktest]
	pub fn vmalloc_alloc_free() {
		reset_page_state();

		let mut list: Vec<AllocInfo> = Vec::new();
		let mut rng = LCG::new(42);

		while let Ok(data) = alloc(((rng.rand() % 31) as usize + 1) * MB) {
			list.push(data);
		}

		let len = list.len();
		for _ in 0..(len * 10) {
			let l = rng.rand() as usize % len;
			let r = rng.rand() as usize % len;
			list.swap(l, r);
		}

		while let Some(data) = list.pop() {
			free(data);
		}
	}
}

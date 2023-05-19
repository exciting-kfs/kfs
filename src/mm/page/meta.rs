use core::marker::{PhantomData, PhantomPinned};
use core::mem::size_of;
use core::ops::IndexMut;
use core::ptr::NonNull;
use core::slice::from_raw_parts_mut;

use crate::boot::PMemory;
use crate::sync::singleton::Singleton;

use crate::mm::{constant::*, util::*};

#[repr(transparent)]
struct MetaData(usize);

#[derive(Clone, Copy)]
struct BitRange {
	pub start: usize,
	pub end: usize,
}

impl BitRange {
	pub const fn new(start: usize, end: usize) -> Self {
		Self { start, end }
	}

	const fn make_mask(idx: usize) -> usize {
		match 1usize.checked_shl(idx as u32) {
			Some(x) => x,
			None => 0,
		}
		.wrapping_sub(1)
	}

	pub const fn mask(&self) -> usize {
		Self::make_mask(self.end) & !Self::make_mask(self.start)
	}
}

impl MetaData {
	pub const INUSE: BitRange = BitRange::new(0, 1);
	pub const RANK: BitRange = BitRange::new(1, 5);
	const UNUSED_AREA: BitRange = BitRange::new(5, PAGE_SHIFT);
	const MAPPED_ADDR: BitRange = BitRange::new(PAGE_SHIFT, usize::BITS as usize);

	pub fn new_unmapped() -> Self {
		MetaData(0)
	}

	pub fn new(mapped_addr: usize) -> Self {
		MetaData(mapped_addr & Self::MAPPED_ADDR.mask())
	}

	pub fn remap(&mut self, new_addr: usize) {
		self.0 = (self.0 & !Self::MAPPED_ADDR.mask()) | (new_addr & Self::MAPPED_ADDR.mask());
	}

	pub fn mapped_addr(&self) -> usize {
		self.0 & Self::MAPPED_ADDR.mask()
	}

	pub fn get_flag(&self, range: BitRange) -> usize {
		(self.0 & range.mask()) >> range.start
	}

	pub fn set_flag(&mut self, range: BitRange, bits: usize) {
		self.0 = (self.0 & !range.mask()) | ((bits << range.start) & range.mask())
	}
}

mod test {
	use super::*;
	use kfs_macro::ktest;

	#[ktest]
	pub fn rank() {
		let mut data = MetaData::new_unmapped();

		for i in 0..=10 {
			data.set_flag(MetaData::RANK, i);
			assert!(data.get_flag(MetaData::RANK) == i);
		}
	}

	#[ktest]
	pub fn new() {
		let addr = pfn_to_addr(42);

		let data = MetaData::new(addr);
		let new_addr = data.mapped_addr();

		assert!(addr == new_addr);
	}

	#[ktest]
	pub fn remap() {
		let mut data = MetaData::new_unmapped();
		assert!(data.mapped_addr() == 0);

		let addr = pfn_to_addr(42);
		data.remap(addr);
		assert!(data.mapped_addr() == addr);
	}
}

#[repr(C, align(4))]
pub struct MetaPage {
	prev: NonNull<MetaPage>,
	next: NonNull<MetaPage>,
	data: MetaData,
	_pin: PhantomPinned,
}

pub struct MetaPageTable;

pub static META_PAGE_TABLE: Singleton<&'static mut [MetaPage]> = Singleton::uninit();

impl MetaPageTable {
	pub unsafe fn alloc(pmem: &mut PMemory) -> (*mut MetaPage, usize) {
		let page_count = addr_to_pfn_64(pmem.linear.end) as usize;

		(pmem.alloc_n::<MetaPage>(page_count), page_count)
	}

	pub unsafe fn init(base_ptr: *mut MetaPage, count: usize) {
		for (pfn, entry) in (0..count).map(|x| (x, base_ptr.add(x))) {
			let vaddr = phys_to_virt(pfn_to_addr(pfn));
			MetaPage::construct_at(entry);
			entry.as_mut().unwrap().remap(vaddr);
		}

		META_PAGE_TABLE.write(from_raw_parts_mut(base_ptr, count));
	}

	pub fn metapage_to_ptr(page: NonNull<MetaPage>) -> NonNull<u8> {
		let index = Self::metapage_to_index(page);

		return unsafe { NonNull::new_unchecked(phys_to_virt(pfn_to_addr(index)) as *mut u8) };
	}

	pub fn ptr_to_metapage(ptr: NonNull<u8>) -> NonNull<MetaPage> {
		let index = addr_to_pfn(virt_to_phys(ptr.as_ptr() as usize));

		return Self::index_to_metapage(index);
	}

	pub fn metapage_to_index(page: NonNull<MetaPage>) -> usize {
		let addr = page.as_ptr() as usize;
		let base = META_PAGE_TABLE.lock().as_ptr() as usize;

		(addr - base) / size_of::<MetaPage>()
	}

	pub fn index_to_metapage(index: usize) -> NonNull<MetaPage> {
		NonNull::from(META_PAGE_TABLE.lock().index_mut(index))
	}
}

macro_rules! metapage_let {
	[$x:ident] => {
		let mut __storage: MaybeUninit<MetaPage> = MaybeUninit::uninit();
		unsafe { MetaPage::construct_at(__storage.as_mut_ptr()) };
		let $x = unsafe { __storage.assume_init_mut() };
	};
}

pub(crate) use metapage_let;

impl MetaPage {
	/// Construct new MetaPage
	pub unsafe fn construct_at(ptr: *mut MetaPage) {
		ptr.write(MetaPage {
			prev: NonNull::new_unchecked(ptr),
			next: NonNull::new_unchecked(ptr),
			data: MetaData::new_unmapped(),
			_pin: PhantomPinned,
		})
	}

	pub fn empty(&self) -> bool {
		self.next == NonNull::from(self)
	}

	pub fn disjoint(&mut self) {
		let mut prev = self.prev;
		let mut next = self.next;
		let mut curr = NonNull::from(self);

		unsafe {
			prev.as_mut().next = next;
			next.as_mut().prev = prev;
			curr.as_mut().prev = curr;
			curr.as_mut().next = curr;
		}
	}

	/// pop node in list (self.next)
	pub fn pop(&mut self) -> Option<NonNull<MetaPage>> {
		if self.empty() {
			return None;
		}

		let next = unsafe { self.next.as_mut() };
		next.disjoint();
		return Some(NonNull::from(next));
	}

	pub fn push(&mut self, mut new_head: NonNull<MetaPage>) {
		unsafe {
			let mut head = NonNull::from(self);
			let mut tail = head.as_mut().prev;
			let mut new_tail = new_head.as_mut().prev;

			tail.as_mut().next = new_head;
			new_head.as_mut().prev = tail;
			new_tail.as_mut().next = head;
			head.as_mut().prev = new_tail;
		}
	}

	/// Split `N` ranked `MetaPage` into two `N - 1` ranked `MetaPage`
	///
	/// # Returns
	/// The tuple of splited pages.
	/// - First element: Reference of left (lower pfn) page.
	/// - Second element: Reference of right (higher pfn) page.
	///
	/// # Safety
	/// - Calling this function when `self.rank == 0` is unsafe.
	///
	/// - If `(self as *mut PagePtr).offset(2 ^ self.rank - 1)`
	/// is invalid address (cannot dereference),
	/// then calling this function is also unsafe (very very... broken case).
	///
	pub unsafe fn split(&mut self) -> (NonNull<MetaPage>, NonNull<MetaPage>) {
		self.rank_down();

		let mut right_page =
			NonNull::new_unchecked((self as *mut Self).offset(rank_to_pages(self.rank()) as isize));

		right_page.as_mut().set_rank(self.rank());

		return (NonNull::from(self), right_page);
	}

	pub unsafe fn merge(&mut self, other: NonNull<MetaPage>) -> NonNull<MetaPage> {
		let mut left_page = match (self as *mut MetaPage) < other.as_ptr() {
			true => NonNull::from(self),
			false => other,
		};

		left_page.as_mut().rank_up();

		return left_page;
	}

	pub fn set_inuse(&mut self, value: bool) {
		self.data.set_flag(MetaData::INUSE, value.into());
	}

	pub fn inuse(&self) -> bool {
		self.data.get_flag(MetaData::INUSE) != 0
	}

	pub fn set_rank(&mut self, rank: usize) {
		self.data.set_flag(MetaData::RANK, rank);
	}

	pub fn rank_down(&mut self) {
		self.set_rank(self.rank() - 1);
	}

	pub fn rank_up(&mut self) {
		self.set_rank(self.rank() + 1);
	}

	pub fn rank(&self) -> usize {
		self.data.get_flag(MetaData::RANK)
	}

	pub fn mapped_addr(&self) -> usize {
		self.data.mapped_addr()
	}

	pub fn remap(&mut self, new_addr: usize) {
		self.data.remap(new_addr);
	}

	pub fn next(&self) -> NonNull<MetaPage> {
		self.next
	}

	pub fn prev(&self) -> NonNull<MetaPage> {
		self.prev
	}
}

pub struct MetaPageIter<'a> {
	head: NonNull<MetaPage>,
	curr: NonNull<MetaPage>,
	_data: PhantomData<&'a MetaPage>,
}

impl<'a> Iterator for MetaPageIter<'a> {
	type Item = &'a MetaPage;

	fn next(&mut self) -> Option<Self::Item> {
		unsafe { self.curr = self.curr.as_mut().next };

		return (self.curr != self.head).then(|| unsafe { self.curr.as_ref() });
	}
}

impl<'a> IntoIterator for &'a MetaPage {
	type Item = &'a MetaPage;
	type IntoIter = MetaPageIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		MetaPageIter {
			head: NonNull::from(self),
			curr: NonNull::from(self),
			_data: PhantomData,
		}
	}
}

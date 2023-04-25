use core::marker::{PhantomData, PhantomPinned};
use core::mem::{align_of, size_of};
use core::ptr::NonNull;
use core::slice::from_raw_parts_mut;

use super::page_allocator::util::{addr_to_pfn, rank_to_pages};
use super::util::current_or_next_aligned;
use super::x86::init::ZoneInfo;

#[repr(C, align(8))]
pub struct MetaPage {
	prev: NonNull<MetaPage>,
	next: NonNull<MetaPage>,
	inuse: bool,
	pub rank: usize,
	_pin: PhantomPinned,
}

pub unsafe fn init_meta_page_table(mem: &mut ZoneInfo) -> &'static mut [MetaPage] {
	let table_start = current_or_next_aligned(mem.normal.start, align_of::<MetaPage>());
	let entry_count = addr_to_pfn(mem.normal.end);
	let table_end = table_start + entry_count * size_of::<MetaPage>();

	assert!(table_end < mem.normal.end);

	mem.normal.start = table_end;

	for pfn in 0..addr_to_pfn(mem.size) {
		let entry = (table_start as *mut MetaPage).add(pfn);
		MetaPage::construct_at(entry);
	}

	return from_raw_parts_mut(table_start as *mut MetaPage, entry_count);
}

impl MetaPage {
	/// Construct new MetaPage
	pub unsafe fn construct_at(ptr: *mut MetaPage) {
		(*ptr).prev = NonNull::new_unchecked(ptr);
		(*ptr).next = NonNull::new_unchecked(ptr);
		(*ptr).rank = 0;
		(*ptr).inuse = false;
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
			curr.as_mut().prev = curr;
			curr.as_mut().next = curr;
			next.as_mut().prev = prev;
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

	pub fn push(&mut self, mut new: NonNull<MetaPage>) {
		unsafe {
			let mut prev = NonNull::from(self);
			let mut next = prev.as_mut().next;

			prev.as_mut().next = new;
			new.as_mut().prev = prev;
			new.as_mut().next = next;
			next.as_mut().prev = new;
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
		self.rank -= 1;

		let mut right_page =
			NonNull::new_unchecked((self as *mut Self).offset(rank_to_pages(self.rank) as isize));

		right_page.as_mut().rank = self.rank;

		return (NonNull::from(self), right_page);
	}

	pub unsafe fn merge(&mut self, other: NonNull<MetaPage>) -> NonNull<MetaPage> {
		let mut left_page = match (self as *mut MetaPage) < other.as_ptr() {
			true => NonNull::from(self),
			false => other,
		};

		left_page.as_mut().rank += 1;

		return left_page;
	}

	pub fn set_inuse(&mut self, value: bool) {
		self.inuse = value;
	}

	pub fn is_inuse(&self) -> bool {
		self.inuse
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

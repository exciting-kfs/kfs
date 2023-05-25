use core::{
	marker::{PhantomData, PhantomPinned},
	ptr::NonNull,
};

use super::metadata::MetaData;
use crate::mm::util::*;

#[derive(Debug)]
#[repr(C, align(4))]
pub struct MetaPage {
	prev: NonNull<MetaPage>,
	next: NonNull<MetaPage>,
	data: MetaData,
	_pin: PhantomPinned,
}

macro_rules! metapage_let {
	[$x:ident] => {
		let mut __storage: core::mem::MaybeUninit<MetaPage> = core::mem::MaybeUninit::uninit();
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
			let mut new = new_head.as_mut();
			// let mut new_tail = new_head.as_mut().prev;

			// tail.as_mut().next = new_head;
			// new_head.as_mut().prev = tail;
			// new_tail.as_mut().next = head;
			// head.as_mut().prev = new_tail;
			tail.as_mut().next = new_head;
			head.as_mut().prev = new_head;
			new.next = head;
			new.prev = tail;
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

	pub fn mapped_addr(&self) -> Option<usize> {
		match self.data.mapped_addr() {
			0 => None,
			x => Some(x),
		}
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

use core::marker::{PhantomData, PhantomPinned};
<<<<<<< HEAD
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use core::slice::from_raw_parts_mut;

use super::boot_alloc;
use super::page_allocator::util::{addr_to_pfn_64, rank_to_pages};
use super::x86::init::VMEMORY;

#[repr(C, align(8))]
pub struct MetaPage {
	prev: NonNull<MetaPage>,
	next: NonNull<MetaPage>,
	inuse: bool,
	pub rank: usize,
	_pin: PhantomPinned,
}

#[repr(transparent)]
pub struct MetaPageTable(&'static mut [MetaPage]);

pub static mut META_PAGE_TABLE: MaybeUninit<MetaPageTable> = MaybeUninit::uninit();

impl MetaPageTable {
	pub unsafe fn init() {
		let page_count = addr_to_pfn_64(VMEMORY.assume_init_ref().normal.end) as usize;

		let base = boot_alloc::alloc_n::<MetaPage>(page_count);
		for entry in (0..page_count).map(|x| base.add(x)) {
			MetaPage::construct_at(entry);
		}

		META_PAGE_TABLE.write(MetaPageTable(from_raw_parts_mut(base, page_count)));
	}
}

impl Deref for MetaPageTable {
	type Target = [MetaPage];

	fn deref(&self) -> &Self::Target {
		self.0
	}
}

impl DerefMut for MetaPageTable {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.0
	}
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
=======
use core::ptr::NonNull;

use super::page_allocator::util::rank_to_pages;

#[repr(C, align(8))]
pub struct MetaPage {
    prev: NonNull<MetaPage>,
    next: NonNull<MetaPage>,
    inuse: bool,
    pub rank: usize,
    _pin: PhantomPinned,
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
>>>>>>> d2cf8c8... WIP: hmm...
}

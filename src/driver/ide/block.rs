use core::{
	alloc::{AllocError, Allocator, Layout},
	cmp::max,
	marker::PhantomData,
	mem::{align_of, size_of, ManuallyDrop},
	ptr::{copy_nonoverlapping, NonNull},
};

use alloc::boxed::Box;

use crate::mm::{
	alloc::phys::Normal,
	constant::{KB, SECTOR_SIZE},
	util::virt_to_phys,
};

pub struct Block<T: ?Sized = ()> {
	ptr: NonNull<[u8]>,
	layout: Layout,
	_p: PhantomData<T>,
}

pub enum Size {
	Sector(usize),
	KB(usize),
}

impl Size {
	fn layout(&self) -> Layout {
		let size = match self {
			Self::Sector(count) => *count * SECTOR_SIZE,
			Self::KB(kb) => *kb * KB / SECTOR_SIZE,
		};

		unsafe { Layout::from_size_align_unchecked(size, SECTOR_SIZE) }
	}
}

impl Block {
	pub fn new(size: Size) -> Result<Self, AllocError> {
		let layout = size.layout();
		let ptr = Normal.allocate(layout)?;

		Ok(Self {
			ptr,
			layout,
			_p: PhantomData,
		})
	}

	pub fn as_chunks(&mut self, size: usize) -> impl Iterator<Item = BlockChunk> {
		debug_assert!(size <= self.ptr.len());

		unsafe { self.ptr.as_mut() }
			.chunks_exact_mut(size)
			.map(|chunk| BlockChunk { chunk })
	}
}

impl<T> Block<[T]> {
	pub unsafe fn as_slice(&mut self, count: usize) -> &mut [T] {
		debug_assert!(max(size_of::<T>(), align_of::<T>()) * count <= self.ptr.len());
		core::slice::from_raw_parts_mut(self.ptr.as_ptr().cast(), count)
	}

	pub unsafe fn into_box_slice(mut self, count: usize) -> Box<[T]> {
		let mut b = Box::new_uninit_slice(count);

		let s = self.as_slice(count).as_mut_ptr();
		let d = b.as_mut_ptr().cast();

		copy_nonoverlapping(s, d, count);
		b.assume_init()
	}
}

impl<T> Block<T> {
	pub fn into<U>(self) -> Block<U> {
		let m = ManuallyDrop::new(self);
		Block {
			ptr: m.ptr,
			layout: m.layout,
			_p: PhantomData,
		}
	}

	pub fn as_phys_addr(&self) -> usize {
		virt_to_phys(self.ptr.as_ptr() as *const u8 as usize)
	}

	pub fn size(&self) -> usize {
		self.ptr.len()
	}

	pub unsafe fn as_one(&mut self) -> &mut T {
		debug_assert!(align_of::<T>() <= self.layout.align(), "invalid align.");
		debug_assert!(
			max(size_of::<T>(), align_of::<T>()) <= self.ptr.len(),
			"invalid size."
		);
		self.ptr.cast().as_mut()
	}

	pub unsafe fn into_box(mut self) -> Box<T> {
		let mut b = Box::new_uninit();

		let s = self.as_one();
		let d = b.as_mut_ptr();

		copy_nonoverlapping(s, d, 1);
		b.assume_init()
	}
}

unsafe impl<#[may_dangle] T: ?Sized> Drop for Block<T> {
	fn drop(&mut self) {
		unsafe { Normal.deallocate(self.ptr.cast(), self.layout) };
	}
}

pub fn layout_of<T>() -> Layout {
	unsafe { Layout::from_size_align_unchecked(size_of::<T>(), align_of::<T>()) }
}

pub struct BlockChunk<'a> {
	chunk: &'a mut [u8],
}

impl<'a> BlockChunk<'a> {
	pub unsafe fn cast<U>(&mut self) -> &mut U {
		let u_size = max(size_of::<U>(), align_of::<U>());
		debug_assert!(u_size <= self.chunk.len());

		&mut *self.chunk.as_mut_ptr().cast()
	}
}

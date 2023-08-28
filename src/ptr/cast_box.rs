use core::{
	alloc::{AllocError, Allocator, Layout},
	cmp::max,
	mem::{align_of, size_of},
	ptr::{copy_nonoverlapping, NonNull},
};

use alloc::boxed::Box;

use crate::mm::{alloc::phys::Normal, util::virt_to_phys};

pub struct CastBox {
	ptr: NonNull<[u8]>,
	layout: Layout,
}

impl CastBox {
	pub fn new(layout: Layout) -> Result<Self, AllocError> {
		let ptr = Normal.allocate(layout)?;

		let layout = unsafe { Layout::from_size_align_unchecked(ptr.len(), ptr.len()) };
		Ok(Self { ptr, layout })
	}

	pub fn as_non_null(&self) -> NonNull<[u8]> {
		self.ptr
	}

	pub fn as_phys_addr(&self) -> usize {
		virt_to_phys(self.ptr.as_ptr() as *const u8 as usize)
	}

	pub fn size(&self) -> usize {
		self.ptr.len()
	}

	pub unsafe fn cast<U>(&mut self) -> &mut U {
		debug_assert!(align_of::<U>() <= self.layout.align(), "invalid align.");
		debug_assert!(
			max(size_of::<U>(), align_of::<U>()) <= self.ptr.len(),
			"invalid size."
		);

		self.ptr.cast().as_mut()
	}

	pub fn as_chunks(&mut self, size: usize) -> impl Iterator<Item = CastChunk> {
		debug_assert!(size <= self.ptr.len());

		let chunks = unsafe { self.ptr.as_mut() }.chunks_exact_mut(size);
		chunks.into_iter().map(|chunk| CastChunk { chunk })
	}

	pub unsafe fn as_slice<U>(&mut self, count: usize) -> &mut [U] {
		debug_assert!(max(size_of::<U>(), align_of::<U>()) * count <= self.ptr.len());
		core::slice::from_raw_parts_mut(self.ptr.as_ptr().cast(), count)
	}

	pub unsafe fn into_box<T>(mut self) -> Box<T> {
		let mut b = Box::new_uninit();

		let s = self.as_slice::<T>(1).as_mut_ptr();
		let d = b.as_mut_ptr();

		copy_nonoverlapping(s, d, 1);
		b.assume_init()
	}

	pub unsafe fn into_box_slice<T>(mut self, count: usize) -> Box<[T]> {
		let mut b = Box::new_uninit_slice(count);

		let s = self.as_slice::<T>(count).as_mut_ptr();
		let d = b.as_mut_ptr().cast();

		copy_nonoverlapping(s, d, count);
		b.assume_init()
	}
}

impl Drop for CastBox {
	fn drop(&mut self) {
		unsafe { Normal.deallocate(self.ptr.cast(), self.layout) };
	}
}

pub fn layout_of<T>() -> Layout {
	unsafe { Layout::from_size_align_unchecked(size_of::<T>(), align_of::<T>()) }
}

pub struct CastChunk<'a> {
	chunk: &'a mut [u8],
}

impl<'a> CastChunk<'a> {
	pub unsafe fn cast<U>(&mut self) -> &mut U {
		let u_size = max(size_of::<U>(), align_of::<U>());
		debug_assert!(u_size <= self.chunk.len());

		&mut *self.chunk.as_mut_ptr().cast()
	}
}

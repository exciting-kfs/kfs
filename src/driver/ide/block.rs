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
	util::{next_align, virt_to_phys},
};

#[derive(Debug)]
pub struct Block<T: ?Sized = ()> {
	ptr: NonNull<[u8]>,
	layout: Layout,
	_p: PhantomData<T>,
}

#[derive(Clone, Copy, Debug)]
pub struct BlockSize {
	kb: usize,
}

impl BlockSize {
	pub const MAX_KB: usize = 64;
	pub const MAX_BYTE: usize = 64 * KB;
	pub const BIGGEST: BlockSize = unsafe { BlockSize::new_unchecked(Self::MAX_BYTE) };

	pub const unsafe fn new_unchecked(bytes: usize) -> BlockSize {
		let kb = next_align(bytes, KB) / KB;
		BlockSize { kb }
	}

	pub const fn from_bytes(bytes: usize) -> Option<BlockSize> {
		let kb = next_align(bytes, KB) / KB;
		if 1 <= kb && kb <= Self::MAX_KB {
			Some(BlockSize { kb })
		} else {
			None
		}
	}

	pub const fn from_sector_count(count: usize) -> Option<BlockSize> {
		Self::from_bytes(count * SECTOR_SIZE)
	}

	#[inline]
	pub fn as_bytes(&self) -> usize {
		self.kb * KB
	}

	pub fn sector_count(&self) -> usize {
		self.as_bytes() / SECTOR_SIZE
	}

	fn layout(&self) -> Layout {
		let size = self.kb * KB;
		unsafe { Layout::from_size_align_unchecked(size, SECTOR_SIZE) }
	}
}

impl Block {
	pub fn new(size: BlockSize) -> Result<Self, AllocError> {
		let layout = size.layout();
		let ptr = Normal.allocate(layout)?;

		Ok(Self {
			ptr,
			layout,
			_p: PhantomData,
		})
	}
}

impl<T> Block<[T]> {
	pub unsafe fn as_slice_mut(&mut self, len: usize) -> &mut [T] {
		debug_assert!(max(size_of::<T>(), align_of::<T>()) * len <= self.ptr.len());
		core::slice::from_raw_parts_mut(self.ptr.as_ptr().cast(), len)
	}

	pub unsafe fn as_slice_ref(&self, count: usize) -> &[T] {
		debug_assert!(max(size_of::<T>(), align_of::<T>()) * count <= self.ptr.len());
		core::slice::from_raw_parts(self.ptr.as_ptr().cast(), count)
	}

	pub unsafe fn into_box_slice(mut self, count: usize) -> Box<[T]> {
		let mut b = Box::new_uninit_slice(count);

		let s = self.as_slice_mut(count).as_mut_ptr();
		let d = b.as_mut_ptr().cast();

		copy_nonoverlapping(s, d, count);
		b.assume_init()
	}
}

impl<T> Block<T> {
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

impl<T: ?Sized> Block<T> {
	pub fn as_phys_addr(&self) -> usize {
		virt_to_phys(self.ptr.as_ptr() as *const u8 as usize)
	}

	#[inline]
	pub fn size(&self) -> usize {
		self.ptr.len()
	}

	pub fn into<U: ?Sized>(self) -> Block<U> {
		let m = ManuallyDrop::new(self);
		Block {
			ptr: m.ptr,
			layout: m.layout,
			_p: PhantomData,
		}
	}

	pub fn as_chunks(&self, size: usize) -> impl Iterator<Item = BlockChunk> {
		debug_assert!(size <= self.ptr.len());

		unsafe { self.ptr.as_ref() }
			.chunks_exact(size)
			.map(|chunk| BlockChunk { chunk })
	}

	pub fn as_chunks_mut(&mut self, size: usize) -> impl Iterator<Item = BlockChunkMut> {
		debug_assert!(size <= self.ptr.len());

		unsafe { self.ptr.as_mut() }
			.chunks_exact_mut(size)
			.map(|chunk| BlockChunkMut { chunk })
	}
}

impl<T: ?Sized> Drop for Block<T> {
	fn drop(&mut self) {
		unsafe { Normal.deallocate(self.ptr.cast(), self.layout) };
	}
}

pub struct BlockChunk<'a> {
	chunk: &'a [u8],
}

impl<'a> BlockChunk<'a> {
	pub unsafe fn cast<U>(&self) -> &U {
		let u_size = max(size_of::<U>(), align_of::<U>());
		debug_assert!(u_size <= self.chunk.len());

		&*self.chunk.as_ptr().cast()
	}
}

pub struct BlockChunkMut<'a> {
	chunk: &'a mut [u8],
}

impl<'a> BlockChunkMut<'a> {
	pub unsafe fn cast<U>(&mut self) -> &mut U {
		let u_size = max(size_of::<U>(), align_of::<U>());
		debug_assert!(u_size <= self.chunk.len());

		&mut *self.chunk.as_mut_ptr().cast()
	}
}

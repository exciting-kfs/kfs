use core::{
	mem::size_of,
	ops::{Deref, DerefMut},
	slice::{from_raw_parts, from_raw_parts_mut},
};

use alloc::collections::LinkedList;

use crate::{mm::util::next_align, syscall::errno::Errno};

use self::{dir_inode::DirInode, record::Record};

use super::{
	block_pool::block::{Slice, SliceMut},
	inode::{self, IterBlockError, ReadIterError},
};

pub mod dir_file;
pub mod dir_inode;
mod record;

struct Iter {
	iter: inode::Iter,
	prev: LinkedList<usize>,
}

impl Iter {
	fn new(inode: &DirInode, cursor: usize) -> Self {
		Self {
			iter: inode::Iter::new(inode.inner().clone(), cursor),
			prev: LinkedList::new(),
		}
	}

	fn dirent_size(&mut self) -> Result<usize, ReadIterError> {
		let prev = self.iter.cursor();

		let record_chunk = self.iter.next(size_of::<Record>())?;

		let total = {
			let slice = record_chunk.slice();
			let record = unsafe { &*slice.as_ptr().cast::<Record>() };
			next_align(record.capacity(), 4)
		};

		self.iter.jump(prev);
		Ok(total)
	}

	fn dirent_size_block(&mut self) -> Result<usize, IterBlockError> {
		let prev = self.iter.cursor();

		let record_chunk = self.iter.next_block(size_of::<Record>())?;

		let total = {
			let slice = record_chunk.slice();
			let record = unsafe { &*slice.as_ptr().cast::<Record>() };
			next_align(record.capacity(), 4)
		};

		self.iter.jump(prev);
		Ok(total)
	}

	fn next(&mut self) -> Result<Dirent, ReadIterError> {
		let prev = self.iter.cursor();
		let total = self.dirent_size()?;
		let chunk = self.iter.next(total)?;

		self.prev.push_front(prev);
		Ok(Dirent { chunk })
	}

	fn next_block(&mut self) -> Result<Dirent, IterBlockError> {
		let prev = self.iter.cursor();
		let total = self.dirent_size_block()?;
		let chunk = self.iter.next_block(total)?;

		self.prev.push_front(prev);
		Ok(Dirent { chunk })
	}

	unsafe fn next_block_unchecked(&mut self) -> Result<Dirent, Errno> {
		let prev = self.iter.cursor();
		let total = self.dirent_size_block().map_err(|e| e.errno_unchecked())?;
		let chunk = self
			.iter
			.next_block(total)
			.map_err(|e| e.errno_unchecked())?;

		self.prev.push_front(prev);
		Ok(Dirent { chunk })
	}

	fn next_mut_block(&mut self) -> Result<DirentMut, IterBlockError> {
		let prev = self.iter.cursor();
		let total = self.dirent_size_block()?;
		let chunk = self
			.iter
			.next_mut_block(total)
			.map_err(|e| IterBlockError::Errno(e))?;

		self.prev.push_front(prev);
		Ok(DirentMut { chunk })
	}

	unsafe fn next_mut_block_unchecked(&mut self) -> Result<DirentMut, Errno> {
		let prev = self.iter.cursor();
		let total = self.dirent_size_block().map_err(|e| e.errno_unchecked())?;
		let chunk = self.iter.next_mut_block(total)?;

		self.prev.push_front(prev);
		Ok(DirentMut { chunk })
	}

	fn rewind(&mut self) {
		if let Some(prev) = self.prev.pop_front() {
			self.iter.jump(prev)
		}
	}

	fn cursor(&self) -> usize {
		self.iter.cursor()
	}
}

#[derive(Debug)]
struct Dirent {
	chunk: inode::Chunk,
}

impl Dirent {
	fn get_record(&self) -> RecordSlice<'_> {
		RecordSlice(self.chunk.slice())
	}

	fn get_name(&self) -> NameSlice<'_> {
		let record = self.get_record();
		let len = record.name_len();
		drop(record);
		NameSlice {
			slice: self.chunk.slice(),
			len,
		}
	}

	fn len(&self) -> usize {
		self.chunk.slice().len()
	}
}

#[derive(Debug)]
struct DirentMut {
	chunk: inode::ChunkMut,
}

impl DirentMut {
	fn get_record(&mut self) -> RecordSliceMut<'_> {
		RecordSliceMut(self.chunk.slice_mut())
	}

	fn get_name(&mut self) -> NameSliceMut<'_> {
		let record = self.get_record();
		let len = record.name_len();
		drop(record);
		NameSliceMut {
			slice: self.chunk.slice_mut(),
			len,
		}
	}

	fn len(&mut self) -> usize {
		self.chunk.slice_mut().len()
	}
}

struct RecordSlice<'a>(Slice<'a>);

impl<'a> Deref for RecordSlice<'a> {
	type Target = Record;
	fn deref(&self) -> &Self::Target {
		unsafe { &*self.0.as_ptr().cast::<Record>() }
	}
}

struct RecordSliceMut<'a>(SliceMut<'a>);

impl<'a> Deref for RecordSliceMut<'a> {
	type Target = Record;
	fn deref(&self) -> &Self::Target {
		unsafe { &*self.0.as_ptr().cast::<Record>() }
	}
}

impl<'a> DerefMut for RecordSliceMut<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.0.as_mut_ptr().cast::<Record>() }
	}
}

struct NameSlice<'a> {
	slice: Slice<'a>,
	len: usize,
}

impl<'a> Deref for NameSlice<'a> {
	type Target = [u8];
	fn deref(&self) -> &Self::Target {
		unsafe {
			let ptr = self.slice.as_ptr().offset(size_of::<Record>() as isize);
			from_raw_parts(ptr, self.len)
		}
	}
}

struct NameSliceMut<'a> {
	slice: SliceMut<'a>,
	len: usize,
}

impl<'a> Deref for NameSliceMut<'a> {
	type Target = [u8];
	fn deref(&self) -> &Self::Target {
		unsafe {
			let ptr = self.slice.as_ptr().offset(size_of::<Record>() as isize);
			from_raw_parts(ptr, self.len)
		}
	}
}

impl<'a> DerefMut for NameSliceMut<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe {
			let ptr = self.slice.as_mut_ptr().offset(size_of::<Record>() as isize);
			from_raw_parts_mut(ptr, self.len)
		}
	}
}

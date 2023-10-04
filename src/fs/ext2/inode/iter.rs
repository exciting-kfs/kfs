use core::{
	ops::{Deref, DerefMut},
	slice::{from_raw_parts, from_raw_parts_mut},
};

use alloc::sync::Arc;

use crate::{
	fs::ext2::Block,
	process::signal::poll_signal_queue,
	scheduler::{
		preempt::AtomicOps,
		sleep::{sleep_and_yield_atomic, Sleep},
	},
	sync::{LockRW, ReadLockGuard, WriteLockGuard},
	syscall::errno::Errno,
};

use super::Inode;

#[derive(Debug)]
pub enum IterError {
	None(AtomicOps),
	End,
	Errno(Errno),
}

impl IterError {
	pub fn downcast_errno(self) -> Option<Errno> {
		match self {
			IterError::Errno(e) => Some(e),
			_ => None,
		}
	}
}

#[macro_export]
macro_rules! handle_iter_error {
	($error: expr, $non_block: expr) => {
		match ($error, $non_block) {
			(IterError::Errno(e), _) => return Err(e),
			(IterError::End, _) => break,
			(IterError::None(_), true) => break,
			(IterError::None(a), false) => {
				crate::scheduler::sleep::sleep_and_yield_atomic(Sleep::Light, a);
				unsafe { poll_signal_queue() }?;
			}
		}
	};
}

#[derive(Debug)]
pub enum IterBlockError {
	End,
	Errno(Errno),
}

impl IterBlockError {
	pub fn errno(self) -> Option<Errno> {
		match self {
			IterBlockError::Errno(e) => Some(e),
			_ => None,
		}
	}
}

impl From<IterError> for IterBlockError {
	fn from(value: IterError) -> Self {
		match value {
			IterError::End => IterBlockError::End,
			IterError::Errno(e) => IterBlockError::Errno(e),
			IterError::None(_) => unreachable!(),
		}
	}
}

pub struct Iter {
	inode: Arc<LockRW<Inode>>,
	cursor: usize,
}

impl Iter {
	pub fn new(inode: Arc<LockRW<Inode>>, cursor: usize) -> Self {
		// pr_debug!("inode data len:{}", inode.read_lock().data.len());
		Self { inode, cursor }
	}

	pub fn write_end(inode: Arc<LockRW<Inode>>) -> Self {
		let cursor = inode.read_lock().info.end_of_blocks();
		Self { inode, cursor }
	}

	pub fn next(&mut self, length: usize) -> Result<Chunk, IterError> {
		let ret = Chunk::new(&self.inode, self.cursor, length)?;
		self.cursor += ret.len;
		Ok(ret)
	}

	pub fn next_block(&mut self, length: usize) -> Result<Chunk, IterBlockError> {
		let ret = Chunk::new_block(&self.inode, self.cursor, length)?;
		self.cursor += ret.len;
		Ok(ret)
	}

	pub fn next_mut(&mut self, length: usize) -> Result<ChunkMut, IterError> {
		let ret = ChunkMut::new(&self.inode, self.cursor, length)?;
		self.cursor += ret.len;

		let mut w_inode = self.inode.write_lock();
		if self.cursor > w_inode.info.get_size() {
			w_inode.info.set_size(self.cursor);
			w_inode.dirty();
		}

		Ok(ret)
	}

	pub fn next_mut_block(&mut self, length: usize) -> Result<ChunkMut, IterBlockError> {
		let ret = ChunkMut::new_block(&self.inode, self.cursor, length)?;
		self.cursor += ret.len;

		let mut w_inode = self.inode.write_lock();
		if self.cursor > w_inode.info.get_size() {
			w_inode.info.set_size(self.cursor);
			w_inode.dirty();
		}

		Ok(ret)
	}

	pub fn cursor(&self) -> usize {
		self.cursor
	}

	pub fn jump(&mut self, cursor: usize) {
		self.cursor = cursor;
	}
}

#[derive(Debug)]
pub struct Chunk {
	chunk: Arc<LockRW<Block>>,
	idx: usize,
	len: usize,
}

impl Chunk {
	fn new(inode: &Arc<LockRW<Inode>>, cursor: usize, length: usize) -> Result<Self, IterError> {
		let r_inode = inode.read_lock();

		if cursor >= r_inode.size() {
			return Err(IterError::End);
		}

		let data = inode.data_read();
		let end = data.slice_end(cursor, length);

		// use crate::pr_debug;
		// pr_debug!("cursor: {}, length: {}, end: {}", cursor, length, end);

		// if let Ok(bid) = w_inode.data.get_bid(cursor) {
		// 	pr_debug!("iter: chunk: new: bid: {}", bid);
		// }

		match data.common().get_chunk(cursor) {
			Ok(chunk) => Ok(Self {
				chunk: chunk.clone(),
				idx: chunk.read_lock().local_index(cursor),
				len: end - cursor,
			}),
			Err(e) => Err(e.handle_read(data, cursor, length)),
		}
	}

	fn new_block(
		inode: &Arc<LockRW<Inode>>,
		cursor: usize,
		length: usize,
	) -> Result<Self, IterBlockError> {
		let mut chunk = Self::new(inode, cursor, length);
		while let Err(IterError::None(atomic)) = chunk {
			sleep_and_yield_atomic(Sleep::Light, atomic);
			unsafe { poll_signal_queue() }.map_err(|e| IterError::Errno(e))?;
			chunk = Self::new(inode, cursor, length);
		}
		chunk.map_err(|e| e.into())
	}

	pub fn slice(&self) -> Slice<'_> {
		Slice {
			chunk_read: self.chunk.read_lock(),
			idx: self.idx,
			len: self.len,
		}
	}
}

#[derive(Debug)]
pub struct ChunkMut {
	chunk: Arc<LockRW<Block>>,
	idx: usize,
	len: usize,
}

impl ChunkMut {
	fn new(inode: &Arc<LockRW<Inode>>, cursor: usize, length: usize) -> Result<Self, IterError> {
		let data = inode.data_write();
		let end = data.slice_mut_end(cursor, length);

		// use crate::pr_debug;
		// pr_debug!("cursor: {}, length: {}, end: {}", cursor, length, end);

		match data.common().get_chunk(cursor) {
			Ok(chunk) => Ok(Self {
				chunk: chunk.clone(),
				idx: cursor % chunk.read_lock().size(),
				len: end - cursor,
			}),
			Err(e) => {
				// pr_debug!("iter: handle_write : {:?}", e);
				Err(e.handle_write(data, cursor, length))
			}
		}
	}

	fn new_block(
		inode: &Arc<LockRW<Inode>>,
		cursor: usize,
		length: usize,
	) -> Result<Self, IterBlockError> {
		let mut chunk = Self::new(inode, cursor, length);
		while let Err(IterError::None(atomic)) = chunk {
			sleep_and_yield_atomic(Sleep::Light, atomic);
			unsafe { poll_signal_queue() }.map_err(|e| IterError::Errno(e))?;
			chunk = Self::new(inode, cursor, length);
		}
		chunk.map_err(|e| e.into())
	}

	pub fn len(&self) -> usize {
		self.len
	}

	pub fn slice_mut(&self) -> SliceMut<'_> {
		SliceMut {
			chunk_write: self.chunk.write_lock(),
			idx: self.idx,
			len: self.len,
		}
	}
}

pub struct Slice<'a> {
	chunk_read: ReadLockGuard<'a, Block>,
	idx: usize,
	len: usize,
}

impl<'a> Deref for Slice<'a> {
	type Target = [u8];
	fn deref(&self) -> &Self::Target {
		unsafe {
			let slice = self.chunk_read.as_slice_ref();
			let ptr = slice.as_ptr().offset(self.idx as isize);
			from_raw_parts(ptr, self.len)
		}
	}
}

pub struct SliceMut<'a> {
	chunk_write: WriteLockGuard<'a, Block>,
	idx: usize,
	len: usize,
}

impl<'a> Deref for SliceMut<'a> {
	type Target = [u8];
	fn deref(&self) -> &Self::Target {
		unsafe {
			let slice = self.chunk_write.as_slice_ref();
			let ptr = slice.as_ptr().offset(self.idx as isize);
			from_raw_parts(ptr, self.len)
		}
	}
}

impl<'a> DerefMut for SliceMut<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe {
			let slice = self.chunk_write.as_slice_mut();
			let ptr = slice.as_mut_ptr().offset(self.idx as isize);
			from_raw_parts_mut(ptr, self.len)
		}
	}
}

use alloc::sync::Arc;

use crate::{
	fs::ext2::{
		block_pool::block::{Slice, SliceMut},
		Block,
	},
	process::signal::poll_signal_queue,
	scheduler::{
		preempt::AtomicOps,
		sleep::{sleep_and_yield_atomic, Sleep},
	},
	sync::LockRW,
	syscall::errno::Errno,
};

use super::Inode;

#[derive(Debug)]
pub enum IterError {
	None(AtomicOps),
	Errno(Errno),
}

#[derive(Debug)]
pub enum ReadIterError {
	None(AtomicOps),
	End,
	Errno(Errno),
}

impl From<IterError> for ReadIterError {
	fn from(value: IterError) -> Self {
		match value {
			IterError::Errno(e) => ReadIterError::Errno(e),
			IterError::None(a) => ReadIterError::None(a),
		}
	}
}

#[macro_export]
macro_rules! handle_r_iter_error {
	($error: expr, $non_block: expr) => {
		match ($error, $non_block) {
			(crate::fs::ext2::inode::iter::ReadIterError::Errno(e), _) => return Err(e),
			(crate::fs::ext2::inode::iter::ReadIterError::End, _) => break,
			(crate::fs::ext2::inode::iter::ReadIterError::None(_), true) => break,
			(crate::fs::ext2::inode::iter::ReadIterError::None(a), false) => {
				crate::scheduler::sleep::sleep_and_yield_atomic(
					crate::scheduler::sleep::Sleep::Light,
					a,
				);
				unsafe { crate::process::signal::poll_signal_queue() }?;
			}
		}
	};
}

#[derive(Debug)]
pub enum WriteIterError {
	None(AtomicOps),
	Errno(Errno),
	Retry,
}

#[macro_export]
macro_rules! handle_w_iter_error {
	($error: expr, $non_block: expr) => {
		match ($error, $non_block) {
			(crate::fs::ext2::inode::iter::WriteIterError::Errno(e), _) => return Err(e),
			(crate::fs::ext2::inode::iter::WriteIterError::Retry, _) => {}
			(crate::fs::ext2::inode::iter::WriteIterError::None(_), true) => break,
			(crate::fs::ext2::inode::iter::WriteIterError::None(a), false) => {
				crate::scheduler::sleep::sleep_and_yield_atomic(Sleep::Light, a);
				unsafe { crate::process::signal::poll_signal_queue() }?;
			}
		}
	};
}

impl From<IterError> for WriteIterError {
	fn from(value: IterError) -> Self {
		match value {
			IterError::Errno(e) => WriteIterError::Errno(e),
			IterError::None(a) => WriteIterError::None(a),
		}
	}
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

	pub unsafe fn errno_unchecked(self) -> Errno {
		match self {
			IterBlockError::Errno(e) => e,
			_ => panic!("please check error state!"),
		}
	}
}

#[macro_export]
macro_rules! handle_iterblock_error {
	($e: expr) => {
		match $e {
			IterBlockError::Errno(e) => return Err(e),
			IterBlockError::End => break,
		}
	};
}

impl From<ReadIterError> for IterBlockError {
	fn from(value: ReadIterError) -> Self {
		match value {
			ReadIterError::End => IterBlockError::End,
			ReadIterError::Errno(e) => IterBlockError::Errno(e),
			ReadIterError::None(_) => unreachable!(),
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

	pub fn next(&mut self, length: usize) -> Result<Chunk, ReadIterError> {
		let ret = Chunk::new(&self.inode, self.cursor, length)?;
		self.cursor += ret.len;
		Ok(ret)
	}

	pub fn next_block(&mut self, length: usize) -> Result<Chunk, IterBlockError> {
		let ret = Chunk::new_block(&self.inode, self.cursor, length)?;
		self.cursor += ret.len;
		Ok(ret)
	}

	pub fn next_mut(&mut self, length: usize) -> Result<ChunkMut, WriteIterError> {
		let ret = ChunkMut::new(&self.inode, self.cursor, length)?;
		self.cursor += ret.len;

		let mut w_inode = self.inode.write_lock();
		if self.cursor > w_inode.info.get_size() {
			w_inode.info.set_size(self.cursor);
			w_inode.dirty();
		}

		Ok(ret)
	}

	pub fn next_mut_block(&mut self, length: usize) -> Result<ChunkMut, Errno> {
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
	fn new(
		inode: &Arc<LockRW<Inode>>,
		cursor: usize,
		length: usize,
	) -> Result<Self, ReadIterError> {
		let r_inode = inode.read_lock();

		if cursor >= r_inode.size() {
			return Err(ReadIterError::End);
		}

		let data = inode.data_read();
		let end = data.slice_end(cursor, length);

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
		while let Err(ReadIterError::None(atomic)) = chunk {
			sleep_and_yield_atomic(Sleep::Light, atomic);
			unsafe { poll_signal_queue() }.map_err(|e| ReadIterError::Errno(e))?;
			chunk = Self::new(inode, cursor, length);
		}
		chunk.map_err(|e| e.into())
	}

	pub fn slice(&self) -> Slice<'_> {
		Slice::new(&self.chunk, self.idx..(self.idx + self.len))
	}
}

#[derive(Debug)]
pub struct ChunkMut {
	chunk: Arc<LockRW<Block>>,
	idx: usize,
	len: usize,
}

impl ChunkMut {
	fn new(
		inode: &Arc<LockRW<Inode>>,
		cursor: usize,
		length: usize,
	) -> Result<Self, WriteIterError> {
		let data = inode.data_write();
		let end = data.slice_mut_end(cursor, length);

		match data.common().get_chunk(cursor) {
			Ok(chunk) => Ok(Self {
				chunk: chunk.clone(),
				idx: cursor % chunk.read_lock().size(),
				len: end - cursor,
			}),
			Err(e) => Err(e.handle_write(data, cursor, length)),
		}
	}

	fn new_block(inode: &Arc<LockRW<Inode>>, cursor: usize, length: usize) -> Result<Self, Errno> {
		let mut chunk = Self::new(inode, cursor, length);
		while let Err(e) = chunk {
			match e {
				WriteIterError::Retry => Ok(()),
				WriteIterError::Errno(e) => Err(e),
				WriteIterError::None(atomic) => {
					sleep_and_yield_atomic(Sleep::Light, atomic);
					unsafe { poll_signal_queue() }
				}
			}?;

			chunk = Self::new(inode, cursor, length);
		}
		Ok(chunk.expect("never WriteIterError occur"))
	}

	pub fn len(&self) -> usize {
		self.len
	}

	pub fn slice_mut(&self) -> SliceMut<'_> {
		SliceMut::new(&self.chunk, self.idx..(self.idx + self.len))
	}
}

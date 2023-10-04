use core::{
	alloc::AllocError,
	cmp::min,
	ops::{Deref, DerefMut},
};

use alloc::{sync::Arc, vec::Vec};

use crate::{
	fs::ext2::block_pool::block::BlockId,
	mm::util::next_align,
	scheduler::preempt::AtomicOps,
	sync::{LockRW, ReadLockGuard, WriteLockGuard},
	syscall::errno::Errno,
};

use super::{info::InodeInfo, iter::IterError, Block, Inode};

pub struct DataCommon<'a> {
	inode: &'a Inode,
}

impl<'a> DataCommon<'a> {
	#[inline]
	pub fn len(&self) -> usize {
		self.inode.chunks.len()
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		self.inode.chunks.is_empty()
	}

	#[inline]
	fn chunk_index(&self, index: usize) -> usize {
		index / self.chunk_size()
	}

	#[inline]
	fn chunk_size(&self) -> usize {
		self.inode.block_size()
	}

	#[inline]
	fn info(&self) -> &InodeInfo {
		&self.inode.info
	}

	fn ready_read(&self, index: usize, length: usize) -> Result<AtomicOps, AllocError> {
		// pr_debug!("data: ready_read: {} {}", index, length);

		let chunks = &self.inode.chunks;
		let sb = &self.inode.sb;

		let begin = self.chunk_index(index);
		let end = self.chunk_index(next_align(index + length, self.chunk_size()));
		let end = min(end, self.len());

		sb.block_pool.load_request(&chunks[begin..end])
	}

	pub fn get_chunk(&self, index: usize) -> Result<Arc<LockRW<Block>>, Error> {
		let chunks = &self.inode.chunks;
		let sb = &self.inode.sb;
		let ci = self.chunk_index(index);

		if ci >= chunks.len() {
			Err(Error::OutOfBound)
		} else {
			let bid = chunks[ci];
			sb.block_pool.get(bid).ok_or(Error::NotLoaded)
		}
	}
}

pub struct DataRead<'a> {
	inode: ReadLockGuard<'a, Inode>,
}

impl<'a> DataRead<'a> {
	pub fn new(inode: ReadLockGuard<'a, Inode>) -> Self {
		Self { inode }
	}

	#[inline]
	pub fn common(&self) -> DataCommon<'_> {
		DataCommon { inode: &self.inode }
	}

	pub fn block_id(&self) -> &Vec<BlockId> {
		&self.inode.chunks
	}

	pub fn slice_end(&self, cursor: usize, length: usize) -> usize {
		let chunk_index = self.common().chunk_index(cursor);
		let chunk_size = self.common().chunk_size();

		let inode_end = self.common().info().get_size();
		let chunk_end = (chunk_index + 1) * chunk_size;
		let request_end = cursor + length;

		min(min(chunk_end, inode_end), request_end)
	}
}

pub struct DataWrite<'a> {
	inode: WriteLockGuard<'a, Inode>,
}

impl<'a> DataWrite<'a> {
	pub fn new(inode: WriteLockGuard<'a, Inode>) -> Self {
		Self { inode }
	}

	pub fn destruct(self) -> WriteLockGuard<'a, Inode> {
		let Self { inode } = self;
		inode
	}

	#[inline]
	pub fn common(&self) -> DataCommon<'_> {
		DataCommon { inode: &self.inode }
	}

	#[inline]
	pub fn block_id_mut(&mut self) -> &mut Vec<BlockId> {
		&mut self.inode.chunks
	}

	#[inline]
	pub fn clear(&mut self) {
		self.inode.chunks.clear();
	}

	#[inline]
	fn info_mut(&mut self) -> InfoMut<'_> {
		InfoMut::new(&mut self.inode)
	}

	pub fn slice_mut_end(&self, cursor: usize, length: usize) -> usize {
		let common = self.common();
		let chunk_index = common.chunk_index(cursor);
		let chunk_size = common.chunk_size();

		let inode_end = common.info().end_of_blocks();
		let chunk_end = (chunk_index + 1) * chunk_size;
		let request_end = cursor + length;

		min(min(chunk_end, inode_end), request_end)
	}

	fn ready_write(&mut self, index: usize, length: usize) -> Result<usize, Errno> {
		let sb = &self.inode.sb;
		let common = self.common();
		let chunk_size = common.chunk_size();

		let count = {
			let start = common.len();
			let end = common.chunk_index(next_align(index + length, chunk_size));
			end.checked_sub(start)
		};

		let mut size = 0;
		if let Some(count) = count {
			let mut bids = sb.reserve_blocks(count)?;
			self.inode.chunks.append(&mut bids);

			size = count * chunk_size;
		}
		Ok(size)
	}
}

#[derive(Debug)]
pub enum Error {
	Alloc,
	NotLoaded,
	OutOfBound,
}

impl Error {
	pub fn handle_read(self, data: DataRead<'_>, index: usize, length: usize) -> IterError {
		// pr_warn!("handle read: {:?}", self);
		match self {
			Self::OutOfBound => IterError::End,
			Self::NotLoaded => Self::ready_read(data.common(), index, length),
			Self::Alloc => IterError::Errno(Errno::ENOMEM),
		}
	}

	pub fn handle_write(self, mut data: DataWrite<'_>, index: usize, length: usize) -> IterError {
		match self {
			Self::OutOfBound => match data.ready_write(index, length) {
				Ok(size) => {
					data.info_mut().inc_blocks(size);
					Self::ready_read(data.common(), index, length)
				}
				Err(e) => IterError::Errno(e),
			},
			Self::NotLoaded => Self::ready_read(data.common(), index, length),
			Self::Alloc => IterError::Errno(Errno::ENOMEM),
		}
	}

	fn ready_read(data: DataCommon<'_>, index: usize, length: usize) -> IterError {
		match data.ready_read(index, length) {
			Ok(atomic) => IterError::None(atomic),
			Err(_) => IterError::Errno(Errno::ENOMEM),
		}
	}
}

struct InfoMut<'a> {
	inode: &'a mut Inode,
}

impl<'a> InfoMut<'a> {
	fn new(inode: &'a mut Inode) -> Self {
		Self { inode }
	}
}

impl<'a> Deref for InfoMut<'a> {
	type Target = InodeInfo;
	fn deref(&self) -> &Self::Target {
		&self.inode.info
	}
}

impl<'a> DerefMut for InfoMut<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inode.info
	}
}

impl<'a> Drop for InfoMut<'a> {
	fn drop(&mut self) {
		self.inode.dirty();
	}
}

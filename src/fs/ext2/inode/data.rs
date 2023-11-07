use core::{
	alloc::AllocError,
	cmp::min,
	ops::{Deref, DerefMut, RangeBounds},
};

use alloc::{
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{
	driver::partition::BlockId,
	mm::util::next_align,
	process::wait_list::WaitList,
	scheduler::preempt::{preempt_disable, AtomicOps},
	sync::{LocalLocked, LockRW, ReadLockGuard, WriteLockGuard},
	syscall::errno::Errno,
	trace_feature,
};

use super::{info::InodeInfo, Block, Inode, IterError, ReadIterError, WriteIterError};

pub enum MaybeChunk {
	Id(BlockId),
	Weak(BlockId, Weak<LockRW<Block>>),
	Loading(BlockId, WaitList),
}

impl MaybeChunk {
	pub fn block_id(&self) -> &BlockId {
		use MaybeChunk::*;
		match self {
			Id(b) => b,
			Loading(b, _) => b,
			Weak(b, _) => b,
		}
	}

	fn not_loaded(&mut self) -> Option<BlockId> {
		use MaybeChunk::*;
		match self {
			Id(b) => {
				let bid = *b;
				*self = Loading(bid, WaitList::new());
				Some(bid)
			}
			Weak(_, w) => w.upgrade().is_none().then(|| {
				let bid = *self.block_id();
				*self = Loading(bid, WaitList::new());
				bid
			}),
			Loading(_, _) => None,
		}
	}

	fn as_block(&mut self) -> Result<Arc<LockRW<Block>>, Error> {
		use MaybeChunk::*;

		match self {
			Id(_) => Err(Error::NotLoaded),
			Weak(_, w) => w.upgrade().ok_or(Error::NotLoaded),
			Loading(_, list) => {
				let atomic = preempt_disable();
				list.register();
				Err(Error::Loading(atomic))
			}
		}
	}
}

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
		let chunks = &self.inode.chunks;
		let sb = &self.inode.sb;

		let begin = self.chunk_index(index);
		let end = self.chunk_index(next_align(index + length, self.chunk_size()));
		let end = min(end, self.len());

		let v = chunks[begin..end]
			.iter()
			.filter_map(|b| b.lock().not_loaded())
			.collect::<Vec<_>>();

		trace_feature!("data-ready_read", "request count: {}", v.len());
		trace_feature!("data-ready_read", "inode: {:?}", self.inode.inum);

		sb.block_pool.load_async(v.as_slice())
	}

	pub fn get_chunk(&self, index: usize) -> Result<Arc<LockRW<Block>>, Error> {
		let chunks = &self.inode.chunks;
		let sb = &self.inode.sb;
		let ci = self.chunk_index(index);

		if ci >= chunks.len() {
			return Err(Error::OutOfBound);
		}

		let mut chunk = chunks[ci].lock();

		chunk.as_block().or_else(|e| {
			let bid = *chunk.block_id();
			let ret = sb.block_pool.get(bid);

			if let Some(b) = ret.as_ref() {
				*chunk = MaybeChunk::Weak(bid, Arc::downgrade(b));
			}

			ret.ok_or(e)
		})
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

	pub fn block_id(&self) -> Vec<BlockId> {
		self.inode
			.chunks
			.iter()
			.map(|b| *b.lock().block_id())
			.collect::<Vec<_>>()
	}

	pub fn slice_end(&self, cursor: usize, length: usize) -> usize {
		let common = self.common();
		let chunk_index = common.chunk_index(cursor);
		let chunk_size = common.chunk_size();

		let inode_end = common.info().get_size();
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

	pub fn truncate(&mut self, length: usize) {
		self.inode.chunks.truncate(length);
	}

	pub fn chunks_range<R: RangeBounds<usize>>(&self, range: R) -> &[LocalLocked<MaybeChunk>] {
		let bound = self.inode.chunks.len();
		&self.inode.chunks[core::slice::range(range, ..bound)]
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
			let blocks = sb.alloc_blocks(count)?;
			self.inode.chunks.extend(blocks.into_iter().map(|b| {
				LocalLocked::new(MaybeChunk::Weak(b.read_lock().id(), Arc::downgrade(&b)))
			}));

			size = count * chunk_size;
		}
		Ok(size)
	}
}

#[derive(Debug)]
pub enum Error {
	Alloc,
	NotLoaded,
	Loading(AtomicOps),
	OutOfBound,
}

impl Error {
	pub fn handle_read(self, data: DataRead<'_>, index: usize, length: usize) -> ReadIterError {
		trace_feature!("inode_iter", "handle_read: {:?}", self);

		match self {
			Self::OutOfBound => ReadIterError::End,
			Self::Loading(a) => ReadIterError::None(a),
			Self::NotLoaded => Self::ready_read(data.common(), index, length).into(),
			Self::Alloc => ReadIterError::Errno(Errno::ENOMEM),
		}
	}

	pub fn handle_write(
		self,
		mut data: DataWrite<'_>,
		index: usize,
		length: usize,
	) -> WriteIterError {
		trace_feature!("inode_iter", "handle_write: {:?}", self);

		match self {
			Self::OutOfBound => match data.ready_write(index, length) {
				Ok(size) => {
					data.info_mut().inc_blocks(size);
					WriteIterError::Retry
				}
				Err(e) => WriteIterError::Errno(e),
			},
			Self::Loading(a) => WriteIterError::None(a),
			Self::NotLoaded => Self::ready_read(data.common(), index, length).into(),
			Self::Alloc => WriteIterError::Errno(Errno::ENOMEM),
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

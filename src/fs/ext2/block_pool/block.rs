use core::{
	cell::UnsafeCell,
	mem::size_of,
	ops::{Deref, DerefMut, Range},
	slice::{from_raw_parts, from_raw_parts_mut},
};

use alloc::sync::{Arc, Weak};

use crate::{
	driver::ide::{block::Block as IdeBlock, dma::hook::WriteBack},
	driver::partition::BlockId,
	sync::{LockRW, ReadLockGuard, WriteLockGuard},
	trace_feature,
};

use super::{list::ListNode, BlockPool};

#[derive(Debug)]
pub struct Block {
	block: IdeBlock<[u8]>,
	node: Arc<BidNode>,
	pool: Weak<BlockPool>,
}

impl Block {
	pub fn new(bid: BlockId, block: IdeBlock, pool: &Arc<BlockPool>) -> Self {
		let node = Arc::new_cyclic(|w| BidNode {
			bid: UnsafeCell::new(bid),
			prev: UnsafeCell::new(w.clone()),
			next: UnsafeCell::new(w.clone()),
		});

		// pr_err!("block: new: bid: {:?}", node.get_bid());

		Self {
			block: block.into(),
			node,
			pool: Arc::downgrade(pool),
		}
	}

	pub unsafe fn new_unregistered(block: IdeBlock) -> Self {
		let node = Arc::new_cyclic(|w| BidNode {
			bid: UnsafeCell::new(BlockId::dangle()),
			prev: UnsafeCell::new(w.clone()),
			next: UnsafeCell::new(w.clone()),
		});

		Self {
			block: block.into(),
			node,
			pool: Weak::default(),
		}
	}

	pub unsafe fn register(&mut self, bid: BlockId, pool: &Arc<BlockPool>) {
		*self.node.bid.get() = bid;

		self.pool = Arc::downgrade(pool);
	}

	pub fn is_unregistered(&self) -> bool {
		self.id() == BlockId::dangle()
	}

	pub(super) fn node(&self) -> Arc<BidNode> {
		self.node.clone()
	}

	#[inline]
	pub fn size(&self) -> usize {
		self.block.size()
	}

	pub fn id(&self) -> BlockId {
		unsafe { *self.node.bid.get() }
	}

	#[inline]
	pub fn local_index(&self, index: usize) -> usize {
		index % self.size()
	}

	fn __as_slice_ref(&self) -> &[u8] {
		unsafe { self.block.as_slice_ref(self.size()) }
	}

	fn __as_slice_mut(&mut self) -> &mut [u8] {
		unsafe { self.block.as_slice_mut(self.size()) }
	}

	fn move_to_back(&self) {
		if let Some(pool) = self.pool.upgrade() {
			trace_feature!("lru-verbose", "block {:?} move to back", self.node.bid());
			pool.lru.lock().move_to_back(self.node.clone())
		}
	}

	fn dirty(&self) {
		let bid = self.id();

		if let Some(pool) = self.pool.upgrade() {
			pool.dirty(bid);
		}
	}
}

impl Drop for Block {
	fn drop(&mut self) {
		trace_feature!(
			"ext2-unmount" | "lru" | "ext2-idspace",
			"block_pool",
			"block: drop: {:?}",
			self.id()
		);

		let node = self.node.clone();

		if !node.is_connected() {
			return;
		}

		if let Some(pool) = self.pool.upgrade() {
			pool.lru.lock().remove(node)
		}
	}
}

impl LockRW<Block> {
	pub fn is_inuse(self: &Arc<Self>) -> bool {
		trace_feature!(
			"lru-verbose",
			"arc block strong count: {}",
			Arc::strong_count(self)
		);
		Arc::strong_count(self) > 1
	}

	pub fn as_slice_ref<'a>(self: &'a Arc<Self>) -> Slice<'a> {
		self.read_lock().move_to_back();

		let size = self.size();
		Slice::new(self, 0..size)
	}

	pub fn as_slice_mut<'a>(self: &'a Arc<Self>) -> SliceMut<'a> {
		self.read_lock().move_to_back();

		let size = self.size();
		SliceMut::new(self, 0..size)
	}

	pub fn as_slice_ref_u32<'a>(self: &'a Arc<Self>) -> Slice32<'a> {
		self.read_lock().move_to_back();

		let size = self.size() / size_of::<u32>();
		Slice32::new(self, 0..size)
	}

	pub fn as_slice_mut_u32<'a>(self: &'a Arc<Self>) -> SliceMut32<'a> {
		self.read_lock().move_to_back();

		let size = self.size() / size_of::<u32>();
		SliceMut32::new(self, 0..size)
	}
}

impl WriteBack for LockRW<Block> {
	fn as_phys_addr(&self) -> usize {
		self.read_lock().block.as_phys_addr()
	}

	fn size(&self) -> usize {
		self.read_lock().block.size()
	}

	fn prepare(&self) {
		unsafe { self.read_lock_manual() };
	}

	fn cleanup(&self) {
		unsafe { self.read_unlock_manual() };
	}
}

#[derive(Debug)]
pub(super) struct BidNode {
	bid: UnsafeCell<BlockId>,
	prev: UnsafeCell<Weak<BidNode>>,
	next: UnsafeCell<Weak<BidNode>>,
}

impl BidNode {
	pub fn bid(&self) -> BlockId {
		unsafe { *self.bid.get() }
	}

	pub fn is_connected(self: &Arc<Self>) -> bool {
		let prev = self.get_prev().upgrade();

		if let Some(p) = prev {
			*p != **self
		} else {
			false
		}
	}
}

impl PartialEq for BidNode {
	fn eq(&self, other: &Self) -> bool {
		unsafe { *self.bid.get() == *other.bid.get() }
	}
}

impl Eq for BidNode {}

unsafe impl Sync for BidNode {}

impl ListNode<BidNode> for BidNode {
	fn get_prev(&self) -> Weak<BidNode> {
		unsafe { (*self.prev.get()).clone() }
	}
	fn get_next(&self) -> Weak<BidNode> {
		unsafe { (*self.next.get()).clone() }
	}

	fn set_prev(&self, prev: Weak<BidNode>) {
		unsafe { (*self.prev.get()) = prev }
	}
	fn set_next(&self, next: Weak<BidNode>) {
		unsafe { (*self.next.get()) = next }
	}
}

pub struct Slice<'a> {
	chunk_read: ReadLockGuard<'a, Block>,
	rng: Range<usize>,
}

impl<'a> Slice<'a> {
	pub fn new(block: &'a Arc<LockRW<Block>>, rng: Range<usize>) -> Self {
		Self {
			chunk_read: block.read_lock(),
			rng,
		}
	}
}

impl<'a> Deref for Slice<'a> {
	type Target = [u8];
	fn deref(&self) -> &Self::Target {
		unsafe {
			self.chunk_read.move_to_back();
			let slice = self.chunk_read.__as_slice_ref();
			let ptr = slice.as_ptr().offset(self.rng.start as isize);
			from_raw_parts(ptr, self.rng.len())
		}
	}
}

pub struct SliceMut<'a> {
	chunk_write: WriteLockGuard<'a, Block>,
	rng: Range<usize>,
}

impl<'a> SliceMut<'a> {
	pub fn new(block: &'a Arc<LockRW<Block>>, rng: Range<usize>) -> Self {
		Self {
			chunk_write: block.write_lock(),
			rng,
		}
	}
}

impl<'a> Deref for SliceMut<'a> {
	type Target = [u8];
	fn deref(&self) -> &Self::Target {
		unsafe {
			self.chunk_write.move_to_back();

			let slice = self.chunk_write.__as_slice_ref();
			let ptr = slice.as_ptr().offset(self.rng.start as isize);
			from_raw_parts(ptr, self.rng.len())
		}
	}
}

impl<'a> DerefMut for SliceMut<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe {
			self.chunk_write.move_to_back();

			let slice = self.chunk_write.__as_slice_mut();
			let ptr = slice.as_mut_ptr().offset(self.rng.start as isize);
			from_raw_parts_mut(ptr, self.rng.len())
		}
	}
}

impl<'a> Drop for SliceMut<'a> {
	fn drop(&mut self) {
		self.chunk_write.dirty();
	}
}

pub struct Slice32<'a> {
	chunk_read: ReadLockGuard<'a, Block>,
	rng: Range<usize>,
}

impl<'a> Slice32<'a> {
	pub fn new(block: &'a Arc<LockRW<Block>>, rng: Range<usize>) -> Self {
		Self {
			chunk_read: block.read_lock(),
			rng,
		}
	}
}

impl<'a> Deref for Slice32<'a> {
	type Target = [u32];
	fn deref(&self) -> &Self::Target {
		unsafe {
			self.chunk_read.move_to_back();

			let slice = self.chunk_read.__as_slice_ref();
			let ptr = (slice.as_ptr() as *const u32).offset(self.rng.start as isize);
			from_raw_parts(ptr, self.rng.len())
		}
	}
}

pub struct SliceMut32<'a> {
	chunk_write: WriteLockGuard<'a, Block>,
	rng: Range<usize>,
}

impl<'a> SliceMut32<'a> {
	pub fn new(block: &'a Arc<LockRW<Block>>, rng: Range<usize>) -> Self {
		Self {
			chunk_write: block.write_lock(),
			rng,
		}
	}
}

impl<'a> Deref for SliceMut32<'a> {
	type Target = [u32];
	fn deref(&self) -> &Self::Target {
		unsafe {
			self.chunk_write.move_to_back();

			let slice = self.chunk_write.__as_slice_ref();
			let ptr = (slice.as_ptr() as *const u32).offset(self.rng.start as isize);
			from_raw_parts(ptr, self.rng.len())
		}
	}
}

impl<'a> DerefMut for SliceMut32<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe {
			self.chunk_write.move_to_back();

			let slice = self.chunk_write.__as_slice_mut();
			let ptr = (slice.as_mut_ptr() as *mut u32).offset(self.rng.start as isize);
			from_raw_parts_mut(ptr, self.rng.len())
		}
	}
}

impl<'a> Drop for SliceMut32<'a> {
	fn drop(&mut self) {
		self.chunk_write.dirty();
	}
}

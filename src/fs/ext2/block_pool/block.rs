use core::{
	cell::UnsafeCell,
	mem::size_of,
	slice::{from_raw_parts, from_raw_parts_mut},
};

use alloc::sync::{Arc, Weak};

use crate::{
	driver::ide::{
		block::{Block as IdeBlock, BlockChunk, BlockChunkMut},
		dma::hook::WriteBack,
	},
	driver::partition::BlockId,
	sync::LockRW,
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

	pub fn as_slice_ref(&self) -> &[u8] {
		self.move_to_back();

		unsafe { self.block.as_slice_ref(self.size()) }
	}

	pub fn as_slice_mut(&mut self) -> &mut [u8] {
		self.dirty();
		self.move_to_back();

		unsafe { self.block.as_slice_mut(self.size()) }
	}

	pub fn as_slice_ref_u32(&self) -> &[u32] {
		self.move_to_back();

		let slice = self.as_slice_ref();
		let ptr = slice.as_ptr();
		let len = slice.len();
		unsafe { from_raw_parts(ptr.cast::<u32>(), len / size_of::<u32>()) }
	}

	pub fn as_slice_mut_u32(&mut self) -> &mut [u32] {
		self.dirty();
		self.move_to_back();

		let slice = self.as_slice_mut();
		let ptr = slice.as_mut_ptr();
		let len = slice.len();
		unsafe { from_raw_parts_mut(ptr.cast::<u32>(), len / size_of::<u32>()) }
	}

	pub fn as_chunks(&self, chunk_size: usize) -> impl Iterator<Item = BlockChunk<'_>> {
		self.move_to_back();
		self.block.as_chunks(chunk_size)
	}

	pub fn as_chunks_mut(&mut self, chunk_size: usize) -> impl Iterator<Item = BlockChunkMut<'_>> {
		self.dirty();
		self.move_to_back();
		self.block.as_chunks_mut(chunk_size)
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

#[cfg(log_level = "debug")]
impl Drop for Block {
	fn drop(&mut self) {
		trace_feature!("ext2-unmount" | "lru", "block: drop: {:?}", self.id());
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

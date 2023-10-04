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
	pr_err,
	sync::LockRW,
};

use super::{list::ListNode, BlockPool};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct BlockId(usize);

impl BlockId {
	#[inline]
	pub fn zero() -> Self {
		BlockId(0)
	}

	pub fn dangle() -> Self {
		BlockId(usize::MAX)
	}

	#[inline]
	pub unsafe fn new_unchecked(id: usize) -> Self {
		BlockId(id)
	}

	#[inline]
	pub fn inner(&self) -> usize {
		self.0
	}

	#[inline]
	pub fn as_u32(&self) -> u32 {
		self.0 as u32
	}
}

#[derive(Debug)]
pub struct Block {
	block: IdeBlock<[u8]>,
	node: Arc<BidNode>,
	pool: Arc<BlockPool>,
}

impl Block {
	pub fn new(bid: BlockId, block: IdeBlock, pool: Arc<BlockPool>) -> Self {
		let node = Arc::new_cyclic(|w| BidNode {
			block_id: UnsafeCell::new(bid),
			prev: UnsafeCell::new(w.clone()),
			next: UnsafeCell::new(w.clone()),
		});

		// pr_err!("block: new: bid: {:?}", node.get_bid());

		Self {
			block: block.into(),
			node,
			pool,
		}
	}

	pub fn node(&self) -> Arc<BidNode> {
		self.node.clone()
	}

	#[inline]
	pub fn size(&self) -> usize {
		self.block.size()
	}

	pub fn id(&self) -> BlockId {
		unsafe { *self.node.block_id.get() }
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
		if self.id() != BlockId::dangle() {
			self.pool.lru.lock().move_to_back(self.node.clone())
		}
	}

	fn dirty(&self) {
		let bid = self.id();

		if bid != BlockId::dangle() {
			self.pool.dirty(bid);
		}
	}
}

impl Drop for Block {
	fn drop(&mut self) {
		pr_err!("block: drop: bid: {:?}", self.id());
		self.pool.lru.lock().remove(self.node.clone());
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
pub struct BidNode {
	block_id: UnsafeCell<BlockId>,
	prev: UnsafeCell<Weak<BidNode>>,
	next: UnsafeCell<Weak<BidNode>>,
}

impl BidNode {
	pub unsafe fn set_bid(&self, bid: BlockId) {
		*self.block_id.get() = bid;
	}
}

impl PartialEq for BidNode {
	fn eq(&self, other: &Self) -> bool {
		unsafe { *self.block_id.get() == *other.block_id.get() }
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

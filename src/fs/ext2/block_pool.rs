mod list;

pub mod block;

use core::{
	alloc::AllocError,
	sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{
	boxed::Box,
	collections::{btree_map::Entry, BTreeMap, BTreeSet},
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{
	driver::ide::block::Block as IdeBlock,
	driver::{ide::dma::hook::Cleanup, partition::BlockId},
	fs::devfs::partition::PartBorrow,
	process::{
		signal::poll_signal_queue,
		task::{Task, CURRENT},
	},
	scheduler::{
		preempt::{preempt_disable, AtomicOps},
		sleep::{sleep_and_yield_atomic, wake_up_deep_sleep, Sleep},
	},
	sync::{LockRW, Locked},
	syscall::errno::Errno,
	trace_feature,
};

use self::{
	block::{BidNode, Block},
	list::List,
};

#[derive(Debug)]
enum MaybeBlock {
	Wait(Vec<Weak<Task>>),
	Block(Arc<LockRW<Block>>),
}

impl MaybeBlock {
	fn into_block(&self) -> Option<Arc<LockRW<Block>>> {
		match self {
			MaybeBlock::Block(b) => Some(b.clone()),
			MaybeBlock::Wait(_) => None,
		}
	}

	fn as_block(&self) -> Option<&Arc<LockRW<Block>>> {
		match self {
			MaybeBlock::Block(b) => Some(b),
			MaybeBlock::Wait(_) => None,
		}
	}
}

#[derive(Debug)]
enum InErr {
	NotLoaded(AtomicOps),
	Wait(AtomicOps),
}

impl InErr {
	fn inner(self) -> AtomicOps {
		match self {
			Self::NotLoaded(a) => a,
			Self::Wait(a) => a,
		}
	}
}

pub enum Error {
	NotLoaded(AtomicOps),
	Alloc,
}

#[derive(Debug)]
pub struct BlockPool {
	dev: PartBorrow,
	pool: Locked<BTreeMap<BlockId, MaybeBlock>>,
	lru: Arc<Locked<List<BidNode>>>,
	dirty: Locked<BTreeSet<BlockId>>,
	registered_count: AtomicUsize,
}

impl BlockPool {
	pub fn new(block_dev: PartBorrow) -> Self {
		Self {
			dev: block_dev,
			pool: Locked::new(BTreeMap::new()),
			lru: Arc::new(Locked::new(List::new())),
			dirty: Locked::new(BTreeSet::new()),
			registered_count: AtomicUsize::new(0),
		}
	}

	#[inline]
	pub fn block_size(&self) -> usize {
		self.dev.block_size().as_bytes()
	}

	pub fn validate_bid(&self, maybe_bid: usize) -> Option<BlockId> {
		self.dev.validate_bid(maybe_bid)
	}

	pub unsafe fn register(self: &Arc<Self>, bid: BlockId, block: Arc<LockRW<Block>>) {
		trace_feature!("block_pool", "block {:?} registered", bid);

		if block.read_lock().is_unregistered() {
			block.write_lock().register(bid, self);

			let mut pool = self.pool.lock();
			let mut lru = self.lru.lock();

			lru.push_back(block.read_lock().node());
			pool.insert(bid, MaybeBlock::Block(block));
			self.registered_count.fetch_add(1, Ordering::Relaxed);

			self.dirty(bid);
		}
	}

	pub unsafe fn unregistered_block(self: &Arc<Self>) -> Result<Arc<LockRW<Block>>, AllocError> {
		trace_feature!("block_pool", "unregistered_block generated");

		let block_size = self.dev.block_size();
		let block = Block::new_unregistered(IdeBlock::new(block_size)?);
		Ok(Arc::new(LockRW::new(block)))
	}

	pub fn dirty(&self, bid: BlockId) {
		self.dirty.lock().insert(bid);
	}

	pub fn sync(&self) {
		let mut dirty = self.dirty.lock();
		while let Some(bid) = dirty.pop_first() {
			if let Some(block) = self.get(bid) {
				self.dev.write_back(bid, block);
			}
		}
	}

	pub fn delete(&self, bid: BlockId) {
		trace_feature!("block_pool", "block {:?} deleted", bid);

		if let Some(block) = self.pool.lock().remove(&bid) {
			if let MaybeBlock::Block(block) = block {
				self.lru.lock().remove(block.read_lock().node());
				self.registered_count.fetch_sub(1, Ordering::Relaxed);
			}
		}
	}

	pub fn get(&self, bid: BlockId) -> Option<Arc<LockRW<Block>>> {
		// pr_debug!("block pool: get: bid: {:?}", bid);
		let pool = self.pool.lock();

		pool.get(&bid).and_then(|e| e.into_block())
	}

	pub fn get_or_load(self: &Arc<Self>, bid: BlockId) -> Result<Arc<LockRW<Block>>, Errno> {
		// pr_debug!("get_or_load: {:?}", bid);

		loop {
			let block = self.get_or_waitlist(bid);

			match block {
				Ok(b) => break Ok(b),
				Err(e) => match e {
					InErr::NotLoaded(a) => {
						let block = self.dev.load_atomic(bid, a)?;
						break Ok(self.insert_block(bid, block));
					}
					InErr::Wait(a) => sleep_and_yield_atomic(Sleep::Light, a),
				},
			}

			unsafe { poll_signal_queue() }?
		}
	}

	pub fn get_or_load_pio(
		self: &Arc<Self>,
		bid: BlockId,
	) -> Result<Arc<LockRW<Block>>, AllocError> {
		let block = self.get_or_waitlist(bid);

		match block {
			Ok(b) => Ok(b),
			Err(_) => {
				let block = self.dev.load_pio(bid)?;
				Ok(self.insert_block(bid, block))
			}
		}
	}

	fn get_or_waitlist(&self, bid: BlockId) -> Result<Arc<LockRW<Block>>, InErr> {
		// pr_debug!("block pool: get_or_waitlist: bid: {:?}", bid);
		let mut pool = self.pool.lock();

		match pool.entry(bid) {
			Entry::Occupied(mut o) => match o.get_mut() {
				MaybeBlock::Block(b) => Ok(b.clone()),
				MaybeBlock::Wait(w) => {
					let atomic = preempt_disable();
					let current = unsafe { CURRENT.get_ref() };
					w.push(Arc::downgrade(current));
					Err(InErr::Wait(atomic))
				}
			},
			Entry::Vacant(v) => {
				let atomic = preempt_disable();
				v.insert(MaybeBlock::Wait(Vec::new()));
				Err(InErr::NotLoaded(atomic))
			}
		}
	}

	pub fn load_request(self: &Arc<Self>, bid: &[BlockId]) -> Result<AtomicOps, AllocError> {
		trace_feature!("block_pool", "load_request: bid: {:?}", bid);

		let mut pool = self.pool.lock();
		let current = unsafe { CURRENT.get_ref() };
		let weak = Arc::downgrade(current);

		let atomic = preempt_disable();

		let bid = bid.iter().filter_map(|bid| match pool.entry(*bid) {
			Entry::Occupied(mut o) => match o.get_mut() {
				MaybeBlock::Block(_) => None,
				MaybeBlock::Wait(w) => {
					w.push(weak.clone());
					None
				}
			},
			Entry::Vacant(v) => {
				let mut list = Vec::new();
				list.push(weak.clone());
				v.insert(MaybeBlock::Wait(list));
				Some(*bid)
			}
		});

		for b in bid {
			let cb = self.async_callback(b);
			self.dev.load_async(b, cb);
		}

		Ok(atomic)
	}

	fn async_callback(self: &Arc<Self>, bid: BlockId) -> Cleanup {
		let this = self.clone();
		let cb = move |result: Result<IdeBlock, AllocError>| {
			if let Ok(block) = result {
				this.insert_block(bid, block);
			} else {
				this.request_retry(bid);
			}
		};

		Box::new(cb)
	}

	pub fn handle_overflow(&self, nr_block_limit: usize) {
		trace_feature!(
			"lru",
			"handle_overflow: registered_count: {}",
			self.registered_count.load(Ordering::Relaxed)
		);

		if self.registered_count.load(Ordering::Relaxed) <= nr_block_limit {
			return;
		}

		let mut pool = self.pool.lock();
		let mut lru = self.lru.lock();

		while let Some(node) = lru.pop_front() {
			let bid = node.bid();

			let is_inuse = pool
				.get(&bid)
				.and_then(|e| e.as_block())
				.filter(|b| b.is_inuse())
				.is_some();

			if is_inuse {
				lru.push_back(node); // push_front?
				break;
			}

			pool.remove(&bid);

			if self.registered_count.fetch_sub(1, Ordering::Relaxed) <= nr_block_limit {
				break;
			}
		}
	}

	fn insert_block(self: &Arc<Self>, bid: BlockId, block: IdeBlock) -> Arc<LockRW<Block>> {
		trace_feature!("block_pool", "block {:?} inserted", bid);

		match self.pool.lock().entry(bid) {
			Entry::Occupied(mut o) => match o.get() {
				MaybeBlock::Block(b) => b.clone(),
				MaybeBlock::Wait(list) => {
					let block = Arc::new(LockRW::new(Block::new(bid, block.into(), self)));
					Self::wake_up_in_list(list);

					*o.get_mut() = MaybeBlock::Block(block.clone());
					self.lru.lock().push_back(block.read_lock().node());
					self.registered_count.fetch_add(1, Ordering::Relaxed);
					block
				}
			},
			Entry::Vacant(_) => panic!("invalid insert block call"),
		}
	}

	fn request_retry(&self, bid: BlockId) {
		let mut pool = self.pool.lock();

		match pool.entry(bid) {
			Entry::Occupied(o) => {
				if let MaybeBlock::Wait(list) = o.get() {
					Self::wake_up_in_list(list);
				}
			}
			Entry::Vacant(_) => panic!("invalid request retry call"),
		}
	}

	fn wake_up_in_list(list: &Vec<Weak<Task>>) {
		list.into_iter().for_each(|task| {
			if let Some(task) = task.upgrade() {
				wake_up_deep_sleep(&task)
			}
		});
	}
}

#[cfg(log_level = "debug")]
impl Drop for BlockPool {
	fn drop(&mut self) {
		trace_feature!("ext2-unmount", "drop: block_pool");
	}
}

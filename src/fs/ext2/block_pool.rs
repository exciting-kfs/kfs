mod list;

pub mod block;

use core::alloc::AllocError;

use alloc::{
	boxed::Box,
	collections::{btree_map::Entry, BTreeMap, BTreeSet},
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{
	driver::ide::{
		block::{Block as IdeBlock, BlockSize},
		dma::{
			dma_req::{ReqInit, ReqWBInit},
			dma_schedule,
			event::DmaInit,
			hook::{OwnHook, WBHook, WriteBack},
			wait_io::WaitIO,
		},
		get_ide_controller,
		ide_id::IdeId,
		lba::LBA28,
		partition::entry::MaybeEntry,
	},
	pr_debug,
	process::{
		signal::poll_signal_queue,
		task::{Task, CURRENT},
	},
	scheduler::{
		preempt::{preempt_disable, AtomicOps},
		sleep::{sleep_and_yield_atomic, wake_up_deep_sleep, Sleep},
	},
	sync::{LockRW, Locked, ReadLockGuard},
	syscall::errno::Errno,
};

use self::{
	block::{BidNode, Block, BlockId},
	list::List,
};

use super::sb::info::SuperBlockInfo;

#[derive(Debug)]
enum MaybeBlock {
	Wait(Vec<Weak<Task>>),
	Block(Arc<LockRW<Block>>),
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
	pool: Locked<BTreeMap<BlockId, MaybeBlock>>,
	block_size: BlockSize,
	ide_id: IdeId,
	entry: ReadLockGuard<'static, MaybeEntry>,
	lru: Arc<Locked<List<BidNode>>>,
	wait_io: WaitIO,
	dirty: Locked<BTreeSet<BlockId>>,
}

impl BlockPool {
	pub fn new(
		ide_id: IdeId,
		entry: ReadLockGuard<'static, MaybeEntry>,
		sb_info: &SuperBlockInfo, // chunk_size: BlockSize,
	) -> Self {
		let block_size = sb_info.block_size();

		Self {
			pool: Locked::new(BTreeMap::new()),
			block_size,
			ide_id,
			entry,
			lru: Arc::new(Locked::new(List::new())),
			wait_io: WaitIO::new(),
			dirty: Locked::new(BTreeSet::new()),
		}
	}

	pub fn block_size(&self) -> BlockSize {
		self.block_size
	}

	pub unsafe fn register(&self, bid: BlockId, block: Arc<LockRW<Block>>) {
		// pr_debug!("block pool: register: bid: {:?}", bid);
		let node = block.read_lock().node();
		let old_bid = block.read_lock().id();
		let mut pool = self.pool.lock();

		if old_bid == BlockId::dangle() {
			unsafe { node.set_bid(bid) };

			self.dirty(bid);
			self.lru.lock().push_back(node);
			pool.insert(bid, MaybeBlock::Block(block));
		} else {
			panic!("invalid usage of block_pool::register");
		}
	}

	pub unsafe fn unregistered_block(self: &Arc<Self>) -> Result<Arc<LockRW<Block>>, AllocError> {
		// pr_debug!("block_pool: unregistered_block");
		let bid = BlockId::dangle();
		let block = Block::new(bid, IdeBlock::new(self.block_size)?, self.clone());
		Ok(Arc::new(LockRW::new(block)))
	}

	pub fn dirty(&self, bid: BlockId) {
		self.dirty.lock().insert(bid);
	}

	pub fn sync(&self) {
		let mut dirty = self.dirty.lock();
		while let Some(bid) = dirty.pop_first() {
			self.__sync(bid);
		}
	}

	pub fn __sync(&self, bid: BlockId) -> Option<()> {
		let chunk = self.get(bid)?;

		// pr_debug!("sync: {:?}", bid);
		let start = self.bid_to_lba(bid);
		let end = start + self.block_size.sector_count();

		let prepare = move || {
			let chunk: Arc<dyn WriteBack> = chunk;
			chunk.prepare();
			chunk
		};

		let cleanup = move |chunk: Arc<dyn WriteBack>| {
			chunk.cleanup();
		};

		let cb = WBHook::new(start, Box::new(prepare), Box::new(cleanup));
		let req = ReqWBInit::new(start..end, cb);
		let event = DmaInit::WriteBack(req);

		dma_schedule(self.ide_id, event);
		Some(())
	}

	pub fn delete(&self, bid: BlockId) {
		pr_debug!("block pool: delete: bid: {:?}", bid);
		self.__sync(bid);
		self.pool.lock().remove(&bid);
	}

	pub fn get(&self, bid: BlockId) -> Option<Arc<LockRW<Block>>> {
		// pr_debug!("block pool: get: bid: {:?}", bid);
		let pool = self.pool.lock();

		pool.get(&bid).and_then(|e| match e {
			MaybeBlock::Wait(_) => None,
			MaybeBlock::Block(b) => Some(b.clone()),
		})
	}

	pub fn get_or_load(self: &Arc<Self>, bid: BlockId) -> Result<Arc<LockRW<Block>>, Errno> {
		// pr_debug!("get_or_load: {:?}", bid);
		loop {
			let block = self.get_or_waitlist(bid);

			match block {
				Ok(b) => break Ok(b),
				Err(e) => match e {
					InErr::NotLoaded(a) => break self.load(bid, a),
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
			Err(_) => self.load_pio(bid),
		}
	}

	fn get_or_waitlist(&self, bid: BlockId) -> Result<Arc<LockRW<Block>>, InErr> {
		// pr_debug!("block pool: get_or_waitlist: bid: {:?}", bid);
		let mut pool = self.pool.lock();

		match pool.entry(bid) {
			Entry::Occupied(mut o) => match o.get_mut() {
				MaybeBlock::Block(b) => Ok(b.clone()),
				MaybeBlock::Wait(w) => {
					// pr_debug!("wait: {:?}", bid);
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
		pr_debug!("load_request: {:?}", bid);
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
			let ev = self.ready_load_async(b);
			dma_schedule(self.ide_id, ev);
		}

		Ok(atomic)
	}

	fn load(
		self: &Arc<Self>,
		bid: BlockId,
		atomic: AtomicOps,
	) -> Result<Arc<LockRW<Block>>, Errno> {
		// pr_warn!("load");
		let event = self.ready_load(bid);
		dma_schedule(self.ide_id, event);
		let block = self.wait_io.wait(atomic)?;
		Ok(self.insert_block(bid, block))
	}

	fn load_pio(self: &Arc<Self>, bid: BlockId) -> Result<Arc<LockRW<Block>>, AllocError> {
		let lba = self.bid_to_lba(bid);
		let size = self.block_size;
		let mut block = IdeBlock::new(size)?.into();

		let raw_sector = unsafe { block.as_slice_mut(size.sector_count()) };

		let ide = get_ide_controller(self.ide_id);
		ide.ata.read_sectors(lba, raw_sector);

		let block = self.insert_block(bid, block.into());

		Ok(block)
	}

	fn ready_load(self: &Arc<Self>, bid: BlockId) -> DmaInit {
		let current = unsafe { CURRENT.get_mut() }.clone();
		let this = self.clone();
		let size = self.block_size;

		let prepare = move || IdeBlock::new(size);
		let cleanup = move |result: Result<IdeBlock, AllocError>| {
			this.wait_io.submit(&current, result);
		};

		let start = self.bid_to_lba(bid);
		let end = start + size.sector_count();

		let cb = OwnHook::new(start, Box::new(prepare), Box::new(cleanup));
		let req = ReqInit::new(start..end, cb);
		DmaInit::Read(req)
	}

	fn ready_load_async(self: &Arc<Self>, bid: BlockId) -> DmaInit {
		let this = self.clone();
		let size = self.block_size;

		let prepare = move || IdeBlock::new(size);
		let cleanup = move |result: Result<IdeBlock, AllocError>| {
			if let Ok(block) = result {
				this.insert_block(bid, block);
			}
		};

		let start = self.bid_to_lba(bid);
		let end = start + size.sector_count();

		let cb = OwnHook::new(start, Box::new(prepare), Box::new(cleanup));
		let req = ReqInit::new(start..end, cb);
		DmaInit::Read(req)
	}

	fn insert_block(self: &Arc<Self>, bid: BlockId, block: IdeBlock) -> Arc<LockRW<Block>> {
		// pr_debug!("block pool: insert_block: bid: {:?}", bid);
		let mut pool = self.pool.lock();

		let block = Arc::new(LockRW::new(Block::new(bid, block.into(), self.clone())));

		match pool.entry(bid) {
			Entry::Occupied(mut o) => {
				if let MaybeBlock::Wait(list) = o.get() {
					list.into_iter().for_each(|t| {
						if let Some(task) = t.upgrade() {
							wake_up_deep_sleep(&task)
						}
					});
					self.lru.lock().push_back(block.read_lock().node());
					*o.get_mut() = MaybeBlock::Block(block.clone());
				}
			}
			Entry::Vacant(_) => panic!("invalid insert block call"),
		}
		block
	}

	fn bid_to_lba(&self, bid: BlockId) -> LBA28 {
		let entry = self.entry.get().unwrap();

		entry.begin().block_size_add(self.block_size, bid.inner())
	}

	pub fn validate_bid(&self, maybe_bid: usize) -> Option<BlockId> {
		let entry = self.entry.get().unwrap();
		let lba = entry.begin().block_size_add(self.block_size, maybe_bid);

		(lba < entry.end()).then_some(unsafe { BlockId::new_unchecked(maybe_bid) })
	}
}

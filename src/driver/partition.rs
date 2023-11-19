mod entry;
mod table;

pub use table::NR_PRIMARY;

use core::alloc::AllocError;

use alloc::{boxed::Box, collections::BTreeMap, sync::Arc};

use crate::{
	process::task::CURRENT,
	scheduler::preempt::{preempt_disable, AtomicOps},
	sync::{LocalLocked, Locked, ReadLockGuard},
	syscall::errno::Errno,
	trace_feature,
};

use self::entry::MaybeEntry;

use super::ide::{
	block::{Block, BlockSize},
	dma::{
		dma_req::{ReqInit, ReqWBInit},
		dma_schedule,
		event::DmaInit,
		hook::{Cleanup, OwnHook, WBHook, WriteBack},
		wait_io::WaitIO,
	},
	get_ide_controller,
	ide_id::{IdeId, NR_IDE_DEV},
	lba::LBA28,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct BlockId(usize);

impl BlockId {
	#[inline]
	pub fn zero() -> Self {
		BlockId(0)
	}

	#[inline]
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

	pub fn index(&self, block_size: usize) -> usize {
		self.inner() - (1024 / block_size)
	}

	#[inline]
	pub fn as_u32(&self) -> u32 {
		self.0 as u32
	}
}

#[derive(Debug)]
pub struct Partition {
	ide_id: IdeId,
	entry: ReadLockGuard<'static, MaybeEntry>,
	wait_io: WaitIO,
	block_size: LocalLocked<BlockSize>,
}

impl Partition {
	const DEFAULT_BLOCK_SIZE: BlockSize = BlockSize::BIGGEST;

	pub fn new(ide_id: IdeId, entry: ReadLockGuard<'static, MaybeEntry>) -> Self {
		Self {
			entry,
			ide_id,
			wait_io: WaitIO::new(),
			block_size: LocalLocked::new(Self::DEFAULT_BLOCK_SIZE),
		}
	}

	pub fn init(&self, block_size: BlockSize) {
		*self.block_size.lock() = block_size;
	}

	pub fn clear(&self) {
		trace_feature!("umount", "partition: clear");
		*self.block_size.lock() = Self::DEFAULT_BLOCK_SIZE;
	}

	pub fn block_size(&self) -> BlockSize {
		self.block_size.lock().clone()
	}

	pub fn load(self: &Arc<Self>, bid: BlockId) -> Result<Block, Errno> {
		trace_feature!("partition-load", "{:?}", bid);
		let atomic = preempt_disable();
		let event = self.ready_load(bid);
		dma_schedule(self.ide_id, event);
		self.wait_io.wait(atomic)
	}

	pub fn load_atomic(self: &Arc<Self>, bid: BlockId, atomic: AtomicOps) -> Result<Block, Errno> {
		trace_feature!("partition-load" | "partition-load_atomic", "{:?}", bid);
		let event = self.ready_load(bid);
		dma_schedule(self.ide_id, event);
		self.wait_io.wait(atomic)
	}

	pub fn load_pio(&self, bid: BlockId) -> Result<Block, AllocError> {
		let size = self.block_size();
		let start = self.bid_to_lba(bid);

		let mut block = Block::new(size)?.into();

		let raw_sector = unsafe { block.as_slice_mut(size.sector_count()) };

		let ide = get_ide_controller(self.ide_id);
		ide.ata.read_sectors(start, raw_sector);

		Ok(block.into())
	}

	pub fn entry_begin(&self) -> LBA28 {
		unsafe { self.entry.get_unchecked().begin() }
	}

	fn ready_load(self: &Arc<Self>, bid: BlockId) -> DmaInit {
		let current = unsafe { CURRENT.get_mut() }.clone();
		let block_size = self.block_size();
		let this = self.clone();

		let prepare = move || Block::new(block_size);
		let cleanup = move |result: Result<Block, AllocError>| {
			this.wait_io.submit(&current, result);
		};

		let start = self.bid_to_lba(bid);
		let end = start + self.block_size().sector_count();
		let cb = OwnHook::new(start, Box::new(prepare), Box::new(cleanup));
		let req = ReqInit::new(start..end, cb);
		DmaInit::Read(req)
	}

	pub fn load_async(&self, bid: BlockId, call_back: Cleanup) {
		trace_feature!("partition-load" | "partition-load_async", "{:?}", bid);
		let ev = self.ready_load_async(bid, call_back);
		dma_schedule(self.ide_id, ev);
	}

	fn ready_load_async(&self, bid: BlockId, call_back: Cleanup) -> DmaInit {
		let block_size = self.block_size();
		let start = self.bid_to_lba(bid);
		let end = start + block_size.sector_count();

		let prepare = move || Block::new(block_size);

		let cb = OwnHook::new(start, Box::new(prepare), call_back);
		let req = ReqInit::new(start..end, cb);
		DmaInit::Read(req)
	}

	pub fn write_back(&self, bid: BlockId, block: Arc<dyn WriteBack>) {
		let start = self.bid_to_lba(bid);
		let end = start + self.block_size().sector_count();

		let prepare = move || {
			let block: Arc<dyn WriteBack> = block;
			block.prepare();
			block
		};

		let cleanup = move |block: Arc<dyn WriteBack>| {
			block.cleanup();
		};

		let cb = WBHook::new(start, Box::new(prepare), Box::new(cleanup));
		let req = ReqWBInit::new(start..end, cb);
		let event = DmaInit::WriteBack(req);

		dma_schedule(self.ide_id, event);
	}

	pub fn validate_bid(&self, maybe_bid: usize) -> Option<BlockId> {
		let entry = unsafe { self.entry.get_unchecked() };
		let lba = entry.begin().block_size_add(self.block_size(), maybe_bid)?;

		(lba < entry.end()).then_some(unsafe { BlockId::new_unchecked(maybe_bid) })
	}

	fn bid_to_lba(&self, bid: BlockId) -> LBA28 {
		let entry = unsafe { self.entry.get_unchecked() };

		unsafe {
			entry
				.begin()
				.block_size_add_unchecked(self.block_size(), bid.inner())
		}
	}
}

pub fn ide_init(devices: [Option<IdeId>; NR_IDE_DEV]) {
	table::init(devices);

	for (i, ent) in table::entrires()
		.enumerate()
		.filter(|(_, e)| e.read_lock().get().is_some())
	{
		let ide_id = unsafe { IdeId::new_unchecked(i / NR_PRIMARY) };
		let dev = Partition::new(ide_id, ent.read_lock());

		trace_feature!("partition", "DEVICE: {:?}\n{:?}", ide_id, unsafe {
			ent.read_lock().get_unchecked()
		});

		BLOCK_DEVICES.lock().insert(i as u8, Arc::new(dev));
	}
}

static BLOCK_DEVICES: Locked<BTreeMap<u8, Arc<Partition>>> = Locked::new(BTreeMap::new());

pub fn get_block_device(num: usize) -> Option<Arc<Partition>> {
	let devices = BLOCK_DEVICES.lock();
	devices.get(&(num as u8)).cloned()
}

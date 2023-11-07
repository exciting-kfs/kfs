mod bitmap;

pub mod bgd;
pub mod info;

use core::{alloc::AllocError, ptr::copy_nonoverlapping};

use alloc::{
	boxed::Box,
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
	vec::Vec,
};

use crate::{
	driver::hpet::get_timestamp_second,
	driver::partition::BlockId,
	fs::vfs,
	mm::util::next_align,
	sync::{LocalLocked, LockRW, Locked},
	syscall::errno::Errno,
	trace_feature,
};

use self::{
	bgd::BGDT,
	bitmap::{BitMap, InGroupBid, InGroupInum},
	info::SuperBlockInfo,
};

use super::{
	block_pool::BlockPool,
	constant::{MAX_CACHED_BLOCK_BYTE, SYNC_INTERVAL},
	inode::{info::InodeInfo, inum::Inum, Inode},
	staged::Staged,
	Block, Ext2,
};
pub struct SuperBlock {
	pub(super) info: LockRW<SuperBlockInfo>,
	pub(super) bgd_table: LocalLocked<BGDT>,
	pub(super) block_pool: Arc<BlockPool>,
	pub(super) inode_cache: Locked<BTreeMap<Inum, Arc<LockRW<Inode>>>>,
	pub(super) dirty_icache: Locked<BTreeSet<Inum>>,
}

impl SuperBlock {
	#[inline]
	pub fn block_size(&self) -> usize {
		self.block_pool.block_size()
	}

	pub fn read_inode_dma(self: &Arc<Self>, inum: Inum) -> Result<Arc<LockRW<Inode>>, Errno> {
		self.__read_inode(inum, |pool, bid| pool.get_or_load(bid))
	}

	pub fn read_inode_pio(self: &Arc<Self>, inum: Inum) -> Result<Arc<LockRW<Inode>>, AllocError> {
		self.__read_inode(inum, |pool, bid| pool.get_or_load_pio(bid))
	}

	fn __read_inode<F, E>(self: &Arc<Self>, inum: Inum, f: F) -> Result<Arc<LockRW<Inode>>, E>
	where
		F: Fn(&Arc<BlockPool>, BlockId) -> Result<Arc<LockRW<Block>>, E>,
	{
		if let Some(inode) = self.inode_cache.lock().get(&inum) {
			trace_feature!("ext2-read_inode", "info: {:x?}", &*inode.info());
			trace_feature!("ext2-read_inode", "from cache: {:?}", inum);
			return Ok(inode.clone());
		}

		let bid = self.inum_to_block_id(inum);
		let block = f(&self.block_pool, bid)?;
		let inode = self.parse_to_inode(inum, block);

		trace_feature!("ext2-read_inode", "info: {:x?}", &*inode.info());
		trace_feature!("ext2-read_inode", "from drive: {:?}", inum);
		self.inode_cache.lock().insert(inum, inode.clone());

		Ok(inode)
	}

	fn parse_to_inode(
		self: &Arc<Self>,
		inum: Inum,
		block: Arc<LockRW<Block>>,
	) -> Arc<LockRW<Inode>> {
		let info = self.info.read_lock();
		let count = info.nr_inode_in_block();
		let local_index = inum.index() % count;

		let mut block = block.write_lock();
		let mut chunk = block
			.as_chunks_mut(info.inode_size())
			.skip(local_index)
			.next()
			.unwrap();

		unsafe {
			let info = chunk.cast::<InodeInfo>();
			Arc::new(LockRW::new(Inode::from_info(inum, info.clone(), self)))
		}
	}

	pub fn dirty_inode(&self, inum: Inum) {
		self.dirty_icache.lock().insert(inum);
	}

	pub fn sync_icache(&self) -> Result<(), Errno> {
		let mut first = self.dirty_icache.lock().first().cloned();
		while let Some(inum) = first {
			let inode = self.inode_cache.lock().get(&inum).cloned();
			if let Some(inode) = inode {
				self.sync_one_icache(&inode)?;
			}

			first = {
				let mut dirty = self.dirty_icache.lock();
				dirty.pop_first();
				dirty.first().cloned()
			}
		}
		Ok(())
	}

	fn sync_one_icache(&self, inode: &Arc<LockRW<Inode>>) -> Result<(), Errno> {
		inode.sync_bid()?;

		let inum = inode.read_lock().inum();
		let bid = self.inum_to_block_id(inum);
		let block = self.block_pool.get_or_load(bid)?;

		let (local_index, inode_size) = {
			let info = self.info.read_lock();
			let local_index = inum.index() % info.nr_inode_in_block();
			(local_index, info.inode_size())
		};

		{
			let mut block = block.write_lock();
			let mut chunk = block
				.as_chunks_mut(inode_size)
				.skip(local_index)
				.next()
				.unwrap();

			let info: &InodeInfo = &inode.info();
			unsafe { copy_nonoverlapping(info, chunk.cast::<InodeInfo>(), 1) };
		}
		Ok(())
	}

	pub fn alloc_inum_staged(self: &Arc<Self>) -> Result<Staged<(), Inum>, Errno> {
		if self.info.read_lock().free_inodes_count() == 0 {
			return Err(Errno::ENOSPC);
		}

		let count = self.info.read_lock().nr_inode_in_group();
		let mut bgdt = self.bgd_table.lock();
		let (bgid, bgd) = bgdt
			.find_bgd(|bgd| bgd.free_inodes_count > 0)
			.ok_or(Errno::ENOSPC)?;

		let block = self.block_pool.get_or_load(bgd.inode_bitmap())?;

		bgd.free_inodes_count -= 1;
		self.info.write_lock().dec_free_inodes_count(1);

		let mut bitmap = BitMap::new(&block, count);
		let sb = self.clone();
		let modify = move |_| {
			let index = bitmap.find_free_space().unwrap();
			bitmap.toggle_bitmap(index);
			sb.info.read_lock().bitmap_index_to_inum(bgid, index)
		};

		let sb = self.clone();
		let restore = move || {
			let mut bgdt = sb.bgd_table.lock();
			let bgd = bgdt.get_bgd_mut(bgid).unwrap();
			bgd.free_inodes_count += 1;
			sb.info.write_lock().inc_free_inodes_count(1);
		};

		Ok(Staged::func_with_restore(modify, restore))
	}

	pub fn alloc_blocks(&self, count: usize) -> Result<Vec<Arc<LockRW<Block>>>, Errno> {
		let block_pool = &self.block_pool;

		let mut blocks = Vec::new();

		for _ in 0..count {
			blocks.push(unsafe { block_pool.unregistered_block()? })
		}

		let bids = self.reserve_blocks(count)?;

		bids.iter()
			.zip(blocks.iter())
			.for_each(|(i, b)| unsafe { self.block_pool.register(*i, b.clone()) });

		Ok(blocks)
	}

	pub fn reserve_blocks(&self, count: usize) -> Result<Vec<BlockId>, Errno> {
		if count >= self.info.read_lock().free_blocks_count() {
			return Err(Errno::ENOSPC);
		}

		let count_in_group = self.info.read_lock().nr_block_in_group();

		let mut bgdt = self.bgd_table.lock();
		let mut groups = bgdt.find_groups(count).ok_or_else(|| Errno::ENOSPC)?;

		let mut bitmaps = Vec::new();
		for bgd in groups.iter() {
			let bitmap = self.block_pool.get_or_load(bgd.block_bitmap())?;
			let bitmap = BitMap::new(&bitmap, count_in_group);
			bitmaps.push(bitmap);
		}

		let mut bids = Vec::new();
		for (bgd, bitmap) in groups.iter_mut().zip(bitmaps.iter_mut()) {
			let free_count = bgd.free_count();
			let indexes = bitmap.find_free_space_multi(free_count).unwrap();
			indexes
				.iter()
				.for_each(|index| bitmap.toggle_bitmap(*index));

			let bid = indexes.into_iter().map(|index| {
				self.info
					.read_lock()
					.bitmap_index_to_block_id(bgd.gid(), index)
			});

			bids.extend(bid);

			bgd.dec_free_blocks_count(free_count);
			self.info.write_lock().dec_free_blocks_count(free_count);
		}
		Ok(bids)
	}

	pub fn dealloc_inum_staged(self: &Arc<Self>, inum: Inum) -> Result<Staged, Errno> {
		let info = self.info.read_lock();

		let inum_in_group = InGroupInum::new(inum, &info);
		let bitmap_bid = {
			let bgdt = self.bgd_table.lock();
			let bgd = bgdt.bgd_of_inum(inum, &info);
			BlockId::from(bgd.inode_bitmap())
		};

		let bitmap = self.block_pool.get_or_load(bitmap_bid)?;
		let mut bitmap = BitMap::new(&bitmap, inum_in_group.count());
		let sb = self.clone();

		Ok(Staged::new(move |_| {
			bitmap.toggle_bitmap(inum_in_group.index_in_group());
			let mut bgdt = sb.bgd_table.lock();
			let bgd = bgdt.bgd_of_inum_mut(inum, &sb.info.read_lock());
			bgd.free_inodes_count += 1;

			sb.info.write_lock().inc_free_inodes_count(1);
			sb.inode_cache.lock().remove(&inum);
		}))
	}

	pub fn dealloc_block_staged(self: &Arc<Self>, bid: BlockId) -> Result<Staged, Errno> {
		self.__dealloc_block_staged(bid, |block_pool, bid| block_pool.get_or_load(bid))
	}

	fn __dealloc_block_staged<F, E>(self: &Arc<Self>, bid: BlockId, f: F) -> Result<Staged, E>
	where
		F: Fn(&Arc<BlockPool>, BlockId) -> Result<Arc<LockRW<Block>>, E>,
	{
		let info = self.info.read_lock();
		let mut bgdt = self.bgd_table.lock();
		let bgd = bgdt.bgd_of_bid_mut(bid, &info);

		let bid_in_grp = InGroupBid::new(bid, &info);
		let bitmap_bid = bgd.block_bitmap();
		let bitmap = f(&self.block_pool, bitmap_bid)?;
		let mut bitmap = BitMap::new(&bitmap, bid_in_grp.count());
		let sb = self.clone();

		Ok(Staged::new(move |_| {
			bitmap.toggle_bitmap(bid_in_grp.index_in_group());

			let mut bgdt = sb.bgd_table.lock();
			let bgd = bgdt.bgd_of_bid_mut(bid, &sb.info.read_lock());
			bgd.free_blocks_count += 1;

			// pr_debug!("dealloc block staged: bid: {:?}", bid);
			sb.info.write_lock().inc_free_blocks_count(1);
			sb.block_pool.delete(bid);
		}))
	}

	fn inum_to_block_id(&self, inum: Inum) -> BlockId {
		let info = self.info.read_lock();
		let bgdt = self.bgd_table.lock();
		let bgd = bgdt.bgd_of_inum(inum, &info);
		bgd.block_of_inode(inum, &info)
	}

	pub fn sb_backup_bid(&self) -> Vec<BlockId> {
		self.__backup_bid(0)
	}

	pub fn bgdt_backup_bid(&self) -> Vec<BlockId> {
		self.__backup_bid(1)
	}

	fn __backup_bid(&self, off: usize) -> Vec<BlockId> {
		let mut v = Vec::new();
		v.reserve(5);

		let mut buf = [0; 5];

		self.info.read_lock().sb_backup_bid(&mut buf);

		for b in buf {
			if let Some(bid) = self.block_pool.validate_bid(b + off) {
				v.push(bid)
			}
		}
		v
	}

	fn sync_self(&self) -> Result<(), Errno> {
		let mut info = self.info.write_lock();
		let sec = get_timestamp_second() as u32;
		let is_expired = (sec - info.wtime) >= SYNC_INTERVAL;

		if is_expired {
			info.wtime = sec;
			drop(info);

			self.sync_info()?;
			self.sync_bgdt()?;
		}

		Ok(())
	}

	fn sync_info(&self) -> Result<(), Errno> {
		let bids = self.sb_backup_bid();
		trace_feature!("ext2-sb-sync", "info: bid list: {:?}", bids);

		for bid in bids {
			let block = self.block_pool.get_or_load(bid)?;
			let mut block = block.write_lock();
			let slice = block.as_slice_mut();

			let dst = if bid.inner() == 0 {
				let dst = &mut slice[1024..];
				dst.as_mut_ptr() as *mut SuperBlockInfo
			} else {
				slice.as_mut_ptr() as *mut SuperBlockInfo
			};

			let src = &*self.info.read_lock();
			unsafe { copy_nonoverlapping(src, dst, 1) }
		}
		Ok(())
	}

	fn sync_bgdt(&self) -> Result<(), Errno> {
		let bids = self.bgdt_backup_bid();
		let table_size = self.info.read_lock().bgdt_size();
		let block_size = self.block_size();
		let block_count = next_align(table_size, block_size) / block_size;

		let mut bgdt_iter = self.bgd_table.iter(block_size);

		trace_feature!("ext2-sb-sync", "bgdt: bid list: {:?}", bids);

		for bid in bids {
			let start = bid.inner();
			let end = start + block_count;
			for bid in (start..end).map(|i| unsafe { BlockId::new_unchecked(i) }) {
				let block = self.block_pool.get_or_load(bid)?;
				let dst = block.write_lock().as_slice_mut().as_mut_ptr().cast();

				if let Some(slice) = bgdt_iter.next() {
					unsafe { copy_nonoverlapping(slice.as_ptr(), dst, slice.len()) }
				} else {
					break;
				}
			}
		}
		Ok(())
	}
}

#[cfg(log_level = "debug")]
impl Drop for SuperBlock {
	fn drop(&mut self) {
		trace_feature!("ext2-unmount", "drop: sb");
	}
}

impl vfs::SuperBlock for SuperBlock {
	fn sync(&self) -> Result<(), Errno> {
		let block_size = self.block_size();

		self.sync_icache()?;
		self.sync_self()?;
		self.block_pool.sync();
		self.block_pool
			.handle_overflow(MAX_CACHED_BLOCK_BYTE / block_size);
		Ok(())
	}

	fn unmount(&self) -> Result<(), Errno> {
		trace_feature!("ext2-unmount", "sb: unmount: uuid: {:x?}", self.id());

		self.info.write_lock().edit_for_unmount();

		self.sync()?;

		self.inode_cache.lock().clear();
		Ok(())
	}

	fn filesystem(&self) -> Box<dyn vfs::FileSystem> {
		Box::new(Ext2)
	}

	fn id(&self) -> Vec<u8> {
		self.info.read_lock().uuid().to_vec()
	}
}

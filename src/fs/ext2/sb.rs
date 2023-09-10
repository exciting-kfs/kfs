use core::{
	alloc::AllocError,
	fmt::{Debug, Display},
	mem::size_of,
};

use alloc::{boxed::Box, collections::BTreeMap, sync::Arc};

use crate::{
	driver::ide::{
		block::{Block, BlockSize},
		dma::{
			call_back::CallBack, dma_req::ReqInit, dma_schedule, event::DmaInit, wait_io::WaitIO,
		},
		get_ide_controller,
		ide_id::IdeId,
		lba::LBA28,
		partition::entry::MaybeEntry,
	},
	fs::vfs,
	process::task::CURRENT,
	sync::{
		lock_rw::{LockRW, ReadLockGuard},
		locked::Locked,
	},
	write_field,
};

use super::{
	bgd::BGDT,
	inode::{Inode, InodeInfo, Inum},
};

#[derive(Debug)]
pub enum Error {
	InvalidInum,
	MemoryAlloc,
	FullBlock,
	FullInode,
}

pub struct SuperBlock {
	pub ide_id: IdeId,
	pub entry: ReadLockGuard<'static, MaybeEntry>,
	pub info: LockRW<SuperBlockInfo>,
	pub bgd_table: LockRW<BGDT>,
	pub inode_cache: Locked<BTreeMap<Inum, Arc<LockRW<Inode>>>>,
	pub wait_io: WaitIO,
}

impl SuperBlock {
	pub fn read_inode_dma(self: &Arc<Self>, inum: Inum) -> Result<Arc<LockRW<Inode>>, Error> {
		let this = self.clone();
		let f = |bid| this.read_block_dma(bid);
		self.read_inode(inum, f)
	}

	pub fn read_inode_pio(self: &Arc<Self>, inum: Inum) -> Result<Arc<LockRW<Inode>>, Error> {
		let this = self.clone();
		let f = |bid| this.read_block_pio(bid);
		self.read_inode(inum, f)
	}

	fn read_inode<F>(self: &Arc<Self>, inum: Inum, f: F) -> Result<Arc<LockRW<Inode>>, Error>
	where
		F: FnOnce(usize) -> Result<Block, AllocError>,
	{
		if let Some(inode) = self.inode_cache.lock().get(&inum) {
			return Ok(inode.clone());
		}

		let bid = self.inum_to_block_id(inum).ok_or(Error::InvalidInum)?;
		let block = f(bid).map_err(|_| Error::MemoryAlloc)?;
		let mut inode = self.parse_to_inode(inum, block);

		let ret = inode.get(&inum).unwrap().clone();
		self.inode_cache.lock().append(&mut inode);

		Ok(ret)
	}

	fn parse_to_inode(&self, inum: Inum, mut block: Block) -> BTreeMap<Inum, Arc<LockRW<Inode>>> {
		let data = self.info.read_lock();
		let count = data.inode_per_block();

		block
			.as_chunks(data.inode_size as usize)
			.map(|mut chunk| unsafe {
				let data = chunk.cast::<InodeInfo>();
				Arc::new(LockRW::new(Inode::new(data.clone())))
			})
			.enumerate()
			.map(|(i, inode)| unsafe {
				let base = inum.index() / count * count;
				(Inum::new_unchecked(base + (i + 1)), inode)
			})
			.collect::<BTreeMap<_, _>>()
	}

	fn read_block_dma(self: &Arc<Self>, block_id: usize) -> Result<Block, AllocError> {
		let current = unsafe { CURRENT.get_mut() }.clone();
		let sb = self.clone();
		let size = self.info.read_lock().block_size();

		let prepare = move || Block::new(size);
		let cleanup = move |result: Result<Block, AllocError>| {
			sb.wait_io.submit(&current, result);
		};

		let start = self.block_id_to_lba(block_id);
		let end = start + size.sector_count();

		let cb = CallBack::new(start, Box::new(prepare), Box::new(cleanup));
		let req = ReqInit::new(start..end, cb);
		let event = DmaInit::Read(req);

		dma_schedule(self.ide_id, event);
		self.wait_io.wait()
	}

	fn read_block_pio(&self, block_id: usize) -> Result<Block, AllocError> {
		let lba = self.block_id_to_lba(block_id);
		let size = self.info.read_lock().block_size();
		let mut mem = Block::new(size)?.into();

		let raw_sector = unsafe { mem.as_slice(size.sector_count()) };

		let ide = get_ide_controller(self.ide_id);
		ide.ata.read_sectors(lba, raw_sector);

		Ok(mem.into())
	}

	fn inum_to_block_id(&self, inum: Inum) -> Option<usize> {
		let data = self.info.read_lock();
		let bgdt = self.bgd_table.read_lock();
		let bgd = bgdt.get_bgd(data.group_id(inum))?;

		Some(bgd.inode_table as usize + data.block_offset_in_table(inum))
	}

	fn block_id_to_lba(&self, block_id: usize) -> LBA28 {
		let block_size = self.info.read_lock().block_size();
		let entry = self.entry.get().unwrap();

		entry.begin().block_size_add(block_size, block_id)
	}
}

impl vfs::SuperBlock for SuperBlock {
	fn sync(&self) {
		todo!()
	}
}

#[derive(Clone)]
#[repr(C)]
pub struct SuperBlockInfo {
	pub inodes_count: u32,
	pub blocks_count: u32,
	pub r_blocks_count: u32,
	pub free_blocks_count: u32,
	pub free_inodes_count: u32,
	pub first_data_block: u32,
	pub log_block_size: u32,
	pub log_frag_size: u32,
	pub blocks_per_group: u32,
	pub frags_per_group: u32,
	pub inodes_per_group: u32,
	pub mtime: u32,
	pub wtime: u32,
	pub mnt_count: u16,
	pub max_mnt_count: u16,
	pub magic: u16,
	pub state: u16,
	pub errors: u16,
	pub minor_rev_level: u16,
	pub lastcheck: u32,
	pub checkinterval: u32,
	pub creator_os: u32,
	pub rev_level: u32,
	pub def_resuid: u16,
	pub def_resgid: u16,
	pub first_ino: u32,
	pub inode_size: u16,
	pub block_group_nr: u16,
	pub feature_compat: u32,
	pub feature_incompat: u32,
	pub feature_ro_compat: u32,
	pub uuid: u128,
	pub volume_name: u128,
	pub last_mounted0: u128,
	pub last_mounted1: u128,
	pub last_mounted2: u128,
	pub last_mounted3: u128,
	pub algo_bitmap: u32,
	pub prealloc_blocks: u8,
	pub prealloc_dir_blocks: u8,
	_pad: u16,
}

impl SuperBlockInfo {
	#[inline]
	pub fn group_count(&self) -> usize {
		((self.blocks_count - 1) / self.blocks_per_group + 1) as usize
	}

	#[inline]
	pub fn block_size(&self) -> BlockSize {
		BlockSize::from_bytes(1024 << self.log_block_size).unwrap()
	}

	#[inline]
	pub fn inode_per_block(&self) -> usize {
		self.block_size().as_bytes() / self.inode_size as usize
	}

	#[inline]
	pub fn group_id(&self, inum: Inum) -> usize {
		inum.index() / self.inodes_per_group as usize
	}

	#[inline]
	pub fn group_local_inode(&self, inum: Inum) -> usize {
		inum.index() % self.inodes_per_group as usize
	}

	#[inline]
	pub fn block_local_inode(&self, inum: Inum) -> usize {
		self.group_local_inode(inum) % self.inode_per_block()
	}

	#[inline]
	pub fn block_offset_in_table(&self, inum: Inum) -> usize {
		self.group_local_inode(inum) / self.inode_per_block()
	}

	pub fn bgdt_lba(&self, part_begin: LBA28) -> LBA28 {
		let block_size = self.block_size();
		part_begin.block_size_add(block_size, 1)
			+ match block_size.as_bytes() {
				1024 => 2,
				_ => 0,
			}
	}
}

impl Debug for SuperBlockInfo {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		Display::fmt(&self, f)
	}
}

impl Display for SuperBlockInfo {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "\n")?;
		write_field!(f, self, inodes_count)?;
		write_field!(f, self, blocks_count)?;
		write_field!(f, self, r_blocks_count)?;
		write_field!(f, self, free_blocks_count)?;
		write_field!(f, self, free_inodes_count)?;
		write_field!(f, self, first_data_block)?;
		write_field!(f, self, log_block_size)?;
		write_field!(f, self, log_frag_size)?;
		write_field!(f, self, blocks_per_group)?;
		write_field!(f, self, frags_per_group)?;
		write_field!(f, self, inodes_per_group)?;
		write_field!(f, self, mtime)?;
		write_field!(f, self, wtime)?;
		write_field!(f, self, mnt_count)?;
		write_field!(f, self, max_mnt_count)?;
		write_field!(f, self, magic)?;
		write_field!(f, self, state)?;
		write_field!(f, self, errors)?;
		write_field!(f, self, minor_rev_level)?;
		write_field!(f, self, lastcheck)?;
		write_field!(f, self, checkinterval)?;
		write_field!(f, self, creator_os)?;
		write_field!(f, self, rev_level)?;
		write_field!(f, self, def_resuid)?;
		write_field!(f, self, def_resgid)?;
		write_field!(f, self, first_ino)?;
		write_field!(f, self, inode_size)?;
		write_field!(f, self, block_group_nr)?;
		write_field!(f, self, feature_compat)?;
		write_field!(f, self, feature_incompat)?;
		write_field!(f, self, feature_ro_compat)?;
		write_field!(f, self, uuid)?;
		write_field!(f, self, volume_name)?;
		write_field!(f, self, last_mounted0)?;
		write_field!(f, self, last_mounted1)?;
		write_field!(f, self, last_mounted2)?;
		write_field!(f, self, last_mounted3)?;
		write_field!(f, self, algo_bitmap)?;
		write_field!(f, self, prealloc_blocks)?;
		write_field!(f, self, prealloc_dir_blocks)?;

		Ok(())
	}
}

struct BitMap {
	inner: Block<[usize]>,
}

impl BitMap {
	pub fn new(block: Block) -> Self {
		BitMap {
			inner: block.into(),
		}
	}

	pub fn find_free_space(&mut self) -> Option<usize> {
		let bitmap = unsafe { self.inner.as_slice(self.inner.size() / size_of::<usize>()) };

		for (i, x) in bitmap.iter().enumerate() {
			let x = *x;
			if x != usize::MAX {
				return Some(i * 32 + x.trailing_ones() as usize);
			}
		}

		None
	}

	fn toggle_bitmap(&mut self, idx: usize) {
		let bitmap = unsafe { self.inner.as_slice(self.inner.size() / size_of::<usize>()) };

		let idx_h = idx / usize::BITS as usize;
		let idx_l = idx % usize::BITS as usize;

		bitmap[idx_h] ^= 1 << idx_l;
	}

	fn into_block(self) -> Block {
		let Self { inner } = self;
		inner.into()
	}
}

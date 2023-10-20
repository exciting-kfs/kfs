mod data;
mod id_space;

pub mod info;
pub mod inum;
pub mod iter;

pub use iter::*;

use alloc::{sync::Arc, vec::Vec};

use crate::{
	driver::partition::BlockId,
	fs::vfs::{self, FileType, Permission},
	sync::{LocalLocked, LockRW},
	syscall::errno::Errno,
	trace_feature,
};

use self::{
	data::{DataRead, DataWrite, MaybeChunk},
	id_space::{IdSapceWrite, IdSpaceAdjust, IdSpaceRead},
	info::{InodeInfo, InodeInfoMut, InodeInfoRef},
	inum::Inum,
};

use super::{
	block_pool::BlockPool, dir::dir_inode::DirInode, file::FileInode, sb::SuperBlock, Block,
};

#[derive(Debug)]
pub enum CastError {
	NotFile,
	NotDir,
}

pub struct Inode {
	info: InodeInfo,
	inum: Inum,
	sb: Arc<SuperBlock>,
	chunks: Vec<LocalLocked<MaybeChunk>>,
	synced_len: usize,
}

impl Inode {
	pub fn new_file(inum: Inum, sb: &Arc<SuperBlock>, perm: Permission) -> Self {
		let info = InodeInfo::new(FileType::Regular, perm);

		Self::from_info(inum, info, sb)
	}

	pub fn from_info(inum: Inum, info: InodeInfo, sb: &Arc<SuperBlock>) -> Self {
		Self {
			info,
			inum,
			sb: sb.clone(),
			chunks: Vec::new(),
			synced_len: 0,
		}
	}

	pub fn new_dir(inum: Inum, sb: &Arc<SuperBlock>, perm: Permission, bid: BlockId) -> Self {
		let info = InodeInfo::new(FileType::Directory, perm);

		Self::with_block(inum, info, sb, bid)
	}

	pub fn new_symlink(inum: Inum, sb: &Arc<SuperBlock>, perm: Permission, bid: BlockId) -> Self {
		let info = InodeInfo::new(FileType::SymLink, perm);

		Self::with_block(inum, info, sb, bid)
	}

	fn with_block(inum: Inum, info: InodeInfo, sb: &Arc<SuperBlock>, bid: BlockId) -> Self {
		let mut chunks = Vec::new();

		chunks.push(LocalLocked::new(MaybeChunk::Id(bid)));

		Self {
			info,
			inum,
			sb: sb.clone(),
			chunks,
			synced_len: 0,
		}
	}

	pub fn inum(&self) -> Inum {
		self.inum
	}

	#[inline]
	pub fn size(&self) -> usize {
		self.info.get_size()
	}

	#[inline]
	pub fn block_size(&self) -> usize {
		self.sb.block_size()
	}

	#[inline]
	pub fn super_block(&self) -> &Arc<SuperBlock> {
		&self.sb
	}

	#[inline]
	pub fn block_pool(&self) -> &Arc<BlockPool> {
		&self.sb.block_pool
	}

	pub fn dirty(&self) {
		let inum = self.inum;
		self.sb.dirty_inode(inum);
	}
}

impl LockRW<Inode> {
	pub fn data_read(&self) -> DataRead<'_> {
		DataRead::new(self.read_lock())
	}

	pub fn data_write(&self) -> DataWrite<'_> {
		DataWrite::new(self.write_lock())
	}

	pub fn info(&self) -> InodeInfoRef<'_> {
		InodeInfoRef::new(self.read_lock())
	}

	pub fn info_mut(&self) -> InodeInfoMut<'_> {
		InodeInfoMut::new(self.write_lock())
	}

	fn id_space_read(&self) -> IdSpaceRead<'_> {
		IdSpaceRead::new(self.read_lock())
	}

	fn id_space_adjust(&self) -> IdSpaceAdjust<'_> {
		IdSpaceAdjust::new(self.write_lock())
	}

	pub fn super_block(&self) -> Arc<SuperBlock> {
		self.read_lock().super_block().clone()
	}

	pub fn downcast_dir(self: Arc<Self>) -> Result<DirInode, CastError> {
		let inode = self.read_lock();
		match inode.info.mode & 0xf000 {
			0x4000 => {
				drop(inode);
				Ok(DirInode::from_inode(self))
			}
			_ => Err(CastError::NotDir),
		}
	}

	pub fn downcast_file(self: Arc<Self>) -> Result<FileInode, CastError> {
		let inode = self.read_lock();
		match inode.info.mode & 0xf000 {
			0x4000 => Err(CastError::NotDir),
			_ => {
				drop(inode);
				Ok(FileInode::from_inode(self))
			}
		}
	}

	pub fn load_bid(self: &Arc<Self>) -> Result<(), Errno> {
		if !self.data_read().common().is_empty() {
			return Ok(());
		}
		// pr_debug!("load_bid {:?}", self.read_lock().info);

		let v = self.id_space_read().read_bid()?;

		let mut w_inode = self.write_lock();
		w_inode.synced_len = v.len();
		w_inode.chunks = v
			.into_iter()
			.map(|id| LocalLocked::new(MaybeChunk::Id(id)))
			.collect::<Vec<_>>();

		trace_feature!("inode-load-bid", "chunks_len: {}", w_inode.chunks.len());

		Ok(())
	}

	pub fn sync(self: &Arc<Self>) -> Result<(), Errno> {
		vfs::SuperBlock::sync(self.super_block().as_ref())
	}

	pub fn sync_bid(self: &Arc<Self>) -> Result<(), Errno> {
		{
			let mut id_space = self.id_space_adjust();
			id_space.adjust()?;

			let mut id_space = IdSapceWrite::from_adjust(id_space);
			id_space.sync_with_data()?;
		}

		Ok(())
	}
}

#[cfg(log_level = "debug")]
impl Drop for Inode {
	fn drop(&mut self) {
		trace_feature!("ext2-unmount", "drop: inode {}", self.inum.ino());
	}
}

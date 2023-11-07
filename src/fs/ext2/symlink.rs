use core::mem::size_of;

use alloc::sync::Arc;

use crate::{
	fs::{
		ext2::inode::{self, IterBlockError},
		path::Path,
		vfs::{self, Permission},
	},
	sync::{LockRW, Locked},
	syscall::errno::Errno,
	trace_feature,
	util::endian,
};

use super::{
	inode::{inum::Inum, Inode},
	sb::SuperBlock,
	Block,
};

pub struct SymLinkInode {
	path: Locked<Option<Path>>,
	inode: Arc<LockRW<Inode>>,
}

impl SymLinkInode {
	pub fn from_inode(inode: Arc<LockRW<Inode>>) -> Self {
		let path = Locked::new(None);

		Self { path, inode }
	}

	pub fn new(target: &[u8], inum: Inum, sb: &Arc<SuperBlock>) -> Arc<Self> {
		let inode = Inode::new_symlink(inum, sb, Permission::all());
		let inode = Arc::new(LockRW::new(inode));

		sb.inode_cache.lock().insert(inum, inode.clone());
		inode.read_lock().dirty();

		let path = Locked::new(None);

		{
			let mut info = inode.info_mut();

			for (i, t) in target.chunks(4).enumerate() {
				info.block[i] = endian::little_u32_from_slice(t);
			}
		}

		Arc::new(Self { path, inode })
	}

	pub fn with_block(
		target: &[u8],
		inum: Inum,
		block: &Arc<LockRW<Block>>,
		sb: &Arc<SuperBlock>,
	) -> Arc<Self> {
		block
			.write_lock()
			.as_slice_mut()
			.iter_mut()
			.zip(target)
			.for_each(|(d, s)| *d = *s);

		let bid = block.read_lock().id();
		let inode = Inode::new_symlink_with_block(inum, sb, Permission::all(), bid);
		let inode = Arc::new(LockRW::new(inode));
		sb.inode_cache.lock().insert(inum, inode.clone());
		inode.read_lock().dirty();

		let path = Locked::new(None);

		Arc::new(Self { path, inode })
	}

	pub fn inner(&self) -> &Arc<LockRW<Inode>> {
		&self.inode
	}

	fn read_from_block(&self) -> Result<Path, Errno> {
		let inode = self.inode.clone();
		let size = inode.read_lock().size();
		let mut iter = inode::iter::Iter::new(inode, 0);

		let chunk = iter.next_block(size);

		if let Ok(chunk) = chunk {
			let path = Path::new(&chunk.slice());
			*self.path.lock() = Some(path.clone());
			Ok(path)
		} else {
			match chunk.unwrap_err() {
				IterBlockError::End => Err(Errno::ENOENT),
				IterBlockError::Errno(e) => Err(e),
			}
		}
	}

	fn read_from_inode(&self) -> Path {
		let mut arr: [u8; 60] = [0; 60];

		let info = self.inode.info();
		let size = info.get_size();
		let len = size / size_of::<u32>();

		for i in 0..len {
			let mut data = info.block[i];

			for j in 0..4 {
				arr[i * 4 + j] = data as u8;
				data = data >> 8;
			}
		}

		let path = Path::new(&arr[..size]);
		*self.path.lock() = Some(path.clone());

		path
	}
}

impl vfs::Inode for SymLinkInode {
	fn stat(&self) -> Result<vfs::RawStat, Errno> {
		Ok(self.inner().info().stat())
	}

	fn chown(&self, _owner: usize, _group: usize) -> Result<(), Errno> {
		Ok(())
	}

	fn chmod(&self, _perm: Permission) -> Result<(), Errno> {
		Ok(())
	}
}

impl vfs::SymLinkInode for SymLinkInode {
	fn target(&self) -> Result<Path, Errno> {
		if let Some(path) = self.path.lock().as_ref() {
			return Ok(path.clone());
		}

		let size = self.inode.info().get_size();

		let path = if size > 60 {
			self.read_from_block()?
		} else {
			self.read_from_inode()
		};

		trace_feature!(
			"ext2-symlink",
			"SYM PATH: {:?}",
			alloc::string::String::from_utf8(path.to_buffer())
		);

		Ok(path)
	}
}

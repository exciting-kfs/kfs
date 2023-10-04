use alloc::sync::Arc;

use crate::{
	fs::{
		ext2::inode::{self, IterBlockError},
		path::Path,
		vfs::{self, Permission},
	},
	sync::{LockRW, Locked},
	syscall::errno::Errno,
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

	pub fn new_shared(
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
		let inode = Inode::new_symlink(inum, sb.clone(), Permission::all(), bid);
		let inode = Arc::new(LockRW::new(inode));
		sb.inode_cache.lock().insert(inum, inode.clone());
		inode.read_lock().dirty();

		let path = Locked::new(None);

		Arc::new(Self { path, inode })
	}

	pub fn inner(&self) -> &Arc<LockRW<Inode>> {
		&self.inode
	}
}

impl vfs::SymLinkInode for SymLinkInode {
	fn target(&self) -> Result<Path, Errno> {
		if let Some(path) = self.path.lock().as_ref() {
			return Ok(path.clone());
		}

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
}

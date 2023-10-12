mod entry;
mod handle;
mod inode;
mod walk;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
pub use entry::*;
pub use handle::*;
pub use inode::*;
pub use walk::*;

use crate::sync::Locked;
use crate::syscall::errno::Errno;

use super::devfs::partition::PartBorrow;

pub trait FileSystem {
	fn unmount(&self, sb: &Arc<dyn SuperBlock>) -> Result<(), Errno> {
		sb.unmount()
	}
}

pub trait MemoryFileSystem<S: SuperBlock, D: DirInode>: FileSystem {
	fn mount() -> Result<(Arc<S>, Arc<D>), Errno>;
}

pub trait PhysicalFileSystem<S: SuperBlock, D: DirInode>: FileSystem {
	fn mount(dev: PartBorrow) -> Result<(Arc<S>, Arc<D>), Errno>;
}

pub trait SuperBlock {
	fn id(&self) -> Vec<u8> {
		Vec::new()
	}

	fn sync(&self) -> Result<(), Errno> {
		Ok(())
	}

	fn unmount(&self) -> Result<(), Errno> {
		Ok(())
	}

	fn filesystem(&self) -> Box<dyn FileSystem>;
}

pub static ROOT_DIR_ENTRY: Locked<Option<Arc<VfsDirEntry>>> = Locked::new(None);

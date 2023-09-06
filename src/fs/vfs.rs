mod entry;
mod handle;
mod inode;
mod walk;

use alloc::sync::Arc;
pub use entry::*;
pub use handle::*;
pub use inode::*;
pub use walk::*;

use crate::sync::locked::Locked;
use crate::syscall::errno::Errno;

pub trait FileSystem<S: SuperBlock, D: DirInode> {
	fn mount() -> Result<(Arc<S>, Arc<D>), Errno>;
}

pub trait SuperBlock {
	fn sync(&self);
}

pub static ROOT_DIR_ENTRY: Locked<Option<Arc<VfsDirEntry>>> = Locked::new(None);

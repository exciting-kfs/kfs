pub mod ext2;
pub mod path;
pub mod syscall;
pub mod vfs;

mod devfs;
mod procfs;
mod tmpfs;

use crate::syscall::errno::Errno;

use alloc::rc::Rc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use tmpfs::TmpFs;
use vfs::{VfsDirEntry, ROOT_DIR_ENTRY};

pub use devfs::init as init_devfs;
pub use procfs::init as init_procfs;
pub use procfs::{change_cwd, create_fd_node, create_task_node, delete_fd_node, delete_task_node};

use self::vfs::FileSystem;

pub fn init() -> Result<(), Errno> {
	let (sb, inode) = TmpFs::mount()?;

	let name = Rc::new(Vec::new());
	let _ = ROOT_DIR_ENTRY.lock().insert(Arc::new_cyclic(|w| {
		VfsDirEntry::new(name, inode, w.clone(), sb, true)
	}));

	Ok(())
}

pub mod path;
pub mod syscall;
pub mod vfs;

pub mod devfs;
mod tmpfs;

use crate::fs::vfs::FileSystem;
use crate::syscall::errno::Errno;
use alloc::{rc::Rc, sync::Arc, vec::Vec};
use tmpfs::TmpFs;

use self::vfs::{VfsDirEntry, ROOT_DIR_ENTRY};

pub fn init() -> Result<(), Errno> {
	let (sb, inode) = TmpFs::mount()?;

	let name = Rc::new(Vec::new());
	let _ = ROOT_DIR_ENTRY.lock().insert(Arc::new_cyclic(|w| {
		VfsDirEntry::new(name, inode, w.clone(), sb, true)
	}));

	Ok(())
}

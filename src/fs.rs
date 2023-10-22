pub mod ext2;
pub mod path;
pub mod syscall;
pub mod vfs;

mod devfs;
mod procfs;
mod tmpfs;

use crate::driver::ide::dma::dma_q;
use crate::fs::devfs::partition::PARTITIONS;
use crate::fs::syscall::do_chdir;
use crate::process::get_init_task;
use crate::syscall::errno::Errno;

use alloc::rc::Rc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use tmpfs::TmpFs;
use vfs::{VfsDirEntry, ROOT_DIR_ENTRY};

pub use devfs::init as init_devfs;
pub use procfs::init as init_procfs;
pub use procfs::{change_cwd, create_fd_node, create_task_node, delete_fd_node, delete_task_node};

use self::ext2::Ext2;
use self::vfs::{MemoryFileSystem, PhysicalFileSystem};

pub fn init_rootfs() -> Result<(), Errno> {
	let (sb, inode) = TmpFs::mount()?;

	let name = Rc::new(Vec::new());
	let _ = ROOT_DIR_ENTRY.lock().insert(Arc::new_cyclic(|w| {
		VfsDirEntry::new(name, inode, w.clone(), sb, true)
	}));

	Ok(())
}

pub fn clean_up() -> Result<(), Errno> {
	ext2::clean_up()?;
	dma_q::wait_idle();
	Ok(())
}

pub fn mount_root() {
	use vfs::VfsInode::*;
	let first_partition = match unsafe { &PARTITIONS }.iter().find_map(|x| x.clone()) {
		Some(Block(x)) => match x.get() {
			Ok(x) => x,
			Err(_) => return,
		},
		_ => return,
	};

	let (sb, inode) = match Ext2::mount(first_partition) {
		Ok(x) => x,
		Err(_) => return,
	};

	let name = Rc::new(Vec::new());
	let _ = ROOT_DIR_ENTRY.lock().insert(Arc::new_cyclic(|w| {
		VfsDirEntry::new(name, inode, w.clone(), sb, true)
	}));

	do_chdir(
		&get_init_task(),
		ROOT_DIR_ENTRY.lock().as_ref().unwrap().clone(),
	)
	.unwrap();
}

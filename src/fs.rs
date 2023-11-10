pub mod devfs;
pub mod ext2;
pub mod path;
pub mod syscall;
pub mod vfs;

mod procfs;
mod sysfs;
mod tmpfs;

use crate::driver::ide::dma::dma_q;
use crate::fs::devfs::partition::PARTITIONS;
use crate::fs::procfs::create_mount_entry;
use crate::fs::syscall::do_chdir;
use crate::process::get_init_task;
use crate::syscall::errno::Errno;

use alloc::format;
use alloc::rc::Rc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use tmpfs::TmpFs;
use vfs::{VfsDirEntry, ROOT_DIR_ENTRY};

pub use devfs::init as init_devfs;
pub use procfs::init as init_procfs;
pub use procfs::{change_cwd, create_fd_node, create_task_node, delete_fd_node, delete_task_node};
pub use sysfs::init as init_sysfs;
pub use sysfs::remove_module_node;

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
	let (idx, first_partition) = match unsafe { &PARTITIONS }
		.iter()
		.enumerate()
		.find_map(|(i, x)| x.clone().map(|x| (i, x)))
	{
		Some((idx, Block(x))) => match x.get() {
			Ok(x) => (idx, x),
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

	create_mount_entry(format!("/dev/part{}", idx + 1).as_bytes(), b"/", b"ext2");

	do_chdir(
		&get_init_task(),
		ROOT_DIR_ENTRY.lock().as_ref().unwrap().clone(),
	)
	.unwrap();
}

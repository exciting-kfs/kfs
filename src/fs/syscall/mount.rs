use alloc::sync::Arc;

use crate::fs::{
	ext2::Ext2,
	path::Path,
	tmpfs::TmpFs,
	vfs::{lookup_entry_nofollow, DirInode, MemoryFileSystem, PhysicalFileSystem, SuperBlock},
};
use crate::mm::user::verify::{verify_path, verify_string};
use crate::process::task::CURRENT;
use crate::syscall::errno::Errno;

pub fn sys_mount(dev_path: usize, mount_point: usize, fs_name: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let dev_path = verify_path(dev_path, current)?;
	let dev_path = Path::new(dev_path);
	let mount_point = verify_path(mount_point, current)?;
	let mount_point = Path::new(mount_point);
	let entry = lookup_entry_nofollow(&mount_point, current).and_then(|x| x.downcast_dir())?;

	let fs_name = verify_string(fs_name, current, 256)?;

	let block_device = {
		let entry = lookup_entry_nofollow(&dev_path, current).and_then(|x| x.downcast_block())?;
		entry.get_device()?
	};

	let (sb, inode): (Arc<dyn SuperBlock>, Arc<dyn DirInode>) = match fs_name {
		b"tmpfs" => {
			let (sb, inode) = TmpFs::mount()?;
			(sb, inode)
		}
		b"ext2" => {
			let (sb, inode) = Ext2::mount(block_device)?;
			(sb, inode)
		}
		_ => return Err(Errno::EINVAL),
	};

	entry.mount(inode, sb, current).map(|_| 0)
}

pub fn sys_umount(path: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);
	let entry = lookup_entry_nofollow(&path, current).and_then(|x| x.downcast_dir())?;

	entry.unmount(current).map(|_| 0)
}

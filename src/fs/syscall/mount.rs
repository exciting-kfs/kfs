use crate::{
	fs::{
		path::Path,
		tmpfs::TmpFs,
		vfs::{lookup_dir_entry, FileSystem},
	},
	process::task::CURRENT,
	syscall::errno::Errno,
};

use super::utils::{verify_path, verify_string};

pub fn sys_mount(path: usize, fs_name: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);
	let entry = lookup_dir_entry(path, current)?;

	let fs_name = verify_string(fs_name, current, 256)?;

	let (sb, inode) = match fs_name {
		b"tmpfs" => TmpFs::mount()?,
		_ => return Err(Errno::EINVAL),
	};

	entry.mount(inode, sb, current).map(|_| 0)
}

pub fn sys_umount(path: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);
	let entry = lookup_dir_entry(path, current)?;

	entry.unmount(current).map(|_| 0)
}
use crate::fs::{
	path::Path,
	tmpfs::TmpFs,
	vfs::{lookup_entry_nofollow, FileSystem},
};
use crate::mm::user::verify::{verify_path, verify_string};
use crate::process::task::CURRENT;
use crate::syscall::errno::Errno;

pub fn sys_mount(path: usize, fs_name: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);
	let entry = lookup_entry_nofollow(&path, current).and_then(|x| x.downcast_dir())?;

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
	let entry = lookup_entry_nofollow(&path, current).and_then(|x| x.downcast_dir())?;

	entry.unmount(current).map(|_| 0)
}

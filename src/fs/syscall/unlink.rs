use core::borrow::Borrow;

use crate::fs::path::Path;
use crate::fs::vfs::lookup_entry_follow_except_last;
use crate::process::task::CURRENT;
use crate::syscall::errno::Errno;

use super::utils::verify_path;

pub fn sys_unlink(path: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let entry = lookup_entry_follow_except_last(&path, current)?;

	let parent_dir = entry.parent_dir(current)?;

	parent_dir.unlink(entry.get_name().borrow(), current)?;

	Ok(0)
}

pub fn sys_rmdir(path: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let path = verify_path(path, &current)?;
	let path = Path::new(path);

	let entry = lookup_entry_follow_except_last(&path, current).and_then(|x| x.downcast_dir())?;

	if entry.is_mount_point() {
		return Err(Errno::EPERM);
	}

	let parent_dir = entry.parent_dir(current)?;

	parent_dir.rmdir(entry.get_name().borrow(), current)?;

	Ok(0)
}

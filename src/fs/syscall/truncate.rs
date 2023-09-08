use crate::{
	fs::{path::Path, vfs::lookup_entry_follow},
	process::task::CURRENT,
	syscall::errno::Errno,
};

use super::utils::verify_path;

pub fn sys_truncate(path: usize, length: isize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let entry = lookup_entry_follow(&path, current).and_then(|x| x.downcast_file())?;

	entry.truncate(length, current).map(|_| 0)
}

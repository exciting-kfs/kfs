use crate::fs::{path::Path, vfs::lookup_entry};
use crate::{process::task::CURRENT, syscall::errno::Errno};

use super::utils::verify_path;

pub fn sys_chown(path: usize, owner: usize, group: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let entry = lookup_entry(path, current)?;

	entry.chown(owner, group, current).map(|_| 0)
}

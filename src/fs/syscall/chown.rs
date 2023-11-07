use crate::fs::vfs::Entry;
use crate::fs::{path::Path, vfs::lookup_entry_follow};
use crate::mm::user::verify::verify_path;
use crate::{process::task::CURRENT, syscall::errno::Errno};

pub fn sys_chown(path: usize, owner: usize, group: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let entry = lookup_entry_follow(&path, current)?;

	entry.chown(owner, group, current).map(|_| 0)
}

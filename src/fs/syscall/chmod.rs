use crate::fs::path::Path;
use crate::fs::vfs::{lookup_entry_follow, Permission};
use crate::mm::user::verify::verify_path;
use crate::{process::task::CURRENT, syscall::errno::Errno};

pub fn sys_chmod(path: usize, perm: u32) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let entry = lookup_entry_follow(&path, current)?;

	entry
		.chmod(Permission::from_bits_truncate(perm), current)
		.map(|_| 0)
}

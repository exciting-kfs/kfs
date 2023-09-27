use crate::fs::path::Path;
use crate::fs::vfs::{lookup_entry_follow, Permission};
use crate::mm::user::verify::verify_path;
use crate::process::task::CURRENT;
use crate::syscall::errno::Errno;

pub fn sys_mkdir(path: usize, perm: u32) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let perm = Permission::from_bits_truncate(perm);

	let path = verify_path(path, current)?;
	let mut path = Path::new(path);

	let new_dir_name = path.pop_component().ok_or(Errno::EEXIST)?;

	let base_dir = lookup_entry_follow(&path, current).and_then(|x| x.downcast_dir())?;

	base_dir.mkdir(&new_dir_name, perm, current)?;

	Ok(0)
}

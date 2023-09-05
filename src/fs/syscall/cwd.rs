use crate::{
	fs::{
		path::Path,
		vfs::{lookup_entry, Permission},
	},
	process::task::CURRENT,
	syscall::errno::Errno,
};

use super::utils::verify_path;

pub fn sys_chdir(path: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let dir = lookup_entry(path, current).and_then(|x| x.downcast_dir())?;

	dir.access(Permission::ANY_EXECUTE, current)?;

	let mut cwd = current
		.get_user_ext()
		.expect("must be user process")
		.lock_cwd();

	*cwd = dir;

	Ok(0)
}

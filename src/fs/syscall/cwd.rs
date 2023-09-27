use crate::fs::change_cwd;
use crate::fs::path::Path;
use crate::fs::vfs::{lookup_entry_follow, Permission};
use crate::{mm::user::verify::verify_path, process::task::CURRENT, syscall::errno::Errno};

pub fn sys_chdir(path: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let dir = lookup_entry_follow(&path, current).and_then(|x| x.downcast_dir())?;

	dir.access(Permission::ANY_EXECUTE, current)?;

	*current
		.get_user_ext()
		.expect("must be user process")
		.lock_cwd() = dir;

	change_cwd(current)?;

	Ok(0)
}

use crate::{
	fs::{path::Path, vfs::lookup_entry_follow},
	process::task::CURRENT,
	syscall::errno::Errno,
};

use super::utils::verify_path;

pub fn sys_symlink(target: usize, name: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let target = verify_path(target, current)?;

	let name = verify_path(name, current)?;
	let mut name = Path::new(name);

	let new_symlink_name = name.pop_component().ok_or(Errno::EEXIST)?;
	let base_dir = lookup_entry_follow(&name, current).and_then(|x| x.downcast_dir())?;

	base_dir.symlink(target, &new_symlink_name, current)?;

	Ok(0)
}

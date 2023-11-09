use alloc::sync::Arc;

use crate::{
	fs::{
		path::Path,
		vfs::{lookup_entry_follow, lookup_entry_nofollow},
	},
	mm::user::verify::verify_path,
	process::task::CURRENT,
	syscall::errno::Errno,
};

pub fn sys_link(old_path: usize, new_path: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let old_path = verify_path(old_path, current)?;
	let old_path = Path::new(old_path);
	let new_path = verify_path(new_path, current)?;
	let mut new_path = Path::new(new_path);

	// hmm..?
	if let Ok(_) = lookup_entry_nofollow(&new_path, current) {
		return Err(Errno::EEXIST);
	}

	let name = new_path.pop_component().ok_or(Errno::EINVAL)?;
	let new_parent_ent = lookup_entry_follow(&new_path, current)?;
	let old_ent = lookup_entry_follow(&old_path, current)?;

	if old_ent.is_dir() {
		return Err(Errno::EPERM);
	}

	let old_sb = old_ent.super_block().ok_or(Errno::EPERM)?;
	let new_sb = new_parent_ent.super_block().ok_or(Errno::EPERM)?;

	if Arc::ptr_eq(old_sb, new_sb) {
		new_parent_ent
			.downcast_dir()?
			.link(old_ent, name.as_slice(), current)
			.map(|_| 0)
	} else {
		Err(Errno::EXDEV)
	}
}

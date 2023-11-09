use core::borrow::Borrow;

use alloc::{sync::Arc, vec::Vec};

use crate::{
	fs::{
		path::Path,
		vfs::{lookup_entry_follow, Entry, VfsDirEntry, VfsEntry},
	},
	mm::user::verify::verify_path,
	process::task::CURRENT,
	syscall::errno::Errno,
};

pub fn sys_link(old_path: usize, new_path: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };
	let (old, new_parent, name) = get_entries(old_path, new_path)?;

	if let Ok(_) = new_parent.lookup(name.as_slice(), current) {
		return Err(Errno::EEXIST);
	}

	if old.is_dir() {
		return Err(Errno::EPERM);
	}

	new_parent.link(&old, name.as_slice(), current).map(|_| 0)
}

pub fn sys_rename(old_path: usize, new_path: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };
	let (old, new_parent, name) = get_entries(old_path, new_path)?;
	let old_parent = old.parent_dir(current)?;

	use VfsEntry::*;
	if let Block(_) | Socket(_) = &old {
		return Err(Errno::EPERM);
	}

	if old.is_mount_point() {
		return Err(Errno::EBUSY);
	}

	if let Ok(new) = new_parent.lookup(name.as_slice(), current) {
		if new.is_mount_point() {
			return Err(Errno::EBUSY);
		}

		match (old.is_dir(), new.is_dir()) {
			(true, false) => Err(Errno::ENOTDIR),
			(false, true) => Err(Errno::EISDIR),
			_ => Ok(()),
		}?;

		new_parent.overwrite(&old, new.get_name().borrow(), current)?;
		old_parent.unlink(old.get_name().borrow(), current)?;
	} else {
		new_parent.link(&old, name.as_slice(), current)?;
		old_parent.unlink(old.get_name().borrow(), current)?;
	}

	Ok(0)
}

fn get_entries(
	old_path: usize,
	new_path: usize,
) -> Result<(VfsEntry, Arc<VfsDirEntry>, Vec<u8>), Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let old_path = verify_path(old_path, current)?;
	let old_path = Path::new(old_path);
	let new_path = verify_path(new_path, current)?;
	let mut new_path = Path::new(new_path);

	let old_ent = lookup_entry_follow(&old_path, current)?;
	let name = new_path.pop_component().ok_or(Errno::EINVAL)?;
	let parent_ent = lookup_entry_follow(&new_path, current).and_then(|e| e.downcast_dir())?;

	let old_sb = old_ent.super_block().ok_or(Errno::EPERM)?;
	let new_sb = parent_ent.super_block();

	if Arc::ptr_eq(old_sb, new_sb) {
		Ok((old_ent, parent_ent, name))
	} else {
		Err(Errno::EXDEV)
	}
}

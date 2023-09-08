use alloc::sync::Arc;

use crate::fs::path::Path;
use crate::fs::vfs::{
	lookup_entry_at_follow, lookup_entry_follow, AccessFlag, CreationFlag, IOFlag, Permission,
	VfsRealEntry,
};

use crate::process::task::{Task, CURRENT};
use crate::syscall::errno::Errno;

use super::utils::verify_path;

fn lookup_or_create(
	mut path: Path,
	creation_flags: CreationFlag,
	perm: Permission,
	task: &Arc<Task>,
) -> Result<VfsRealEntry, Errno> {
	let file = path.pop_component();

	let base_entry = lookup_entry_follow(&path, task)?;

	let entry = match file {
		Some(ref name) => base_entry
			.clone()
			.downcast_dir()
			.and_then(|dir| lookup_entry_at_follow(dir, &Path::new(name), task)),
		None => Ok(base_entry.clone()),
	};

	match entry {
		// directory / file already exists.
		Ok(ent) => match creation_flags.contains(CreationFlag::O_EXCL) {
			true => Err(Errno::EEXIST),
			false => Ok(ent),
		},
		Err(e) => match e {
			// not exist. create it
			Errno::ENOENT => base_entry.downcast_dir().and_then(|dir| {
				let file = dir.create(&file.unwrap(), perm, task)?;

				Ok(VfsRealEntry::File(file))
			}),
			// other errors (EPERM, EACCESS, ....)
			_ => Err(e),
		},
	}
}

pub fn sys_open(path: usize, flags: i32, perm: u32) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let access_flags = AccessFlag::from_bits_truncate(flags);
	let creation_flags = CreationFlag::from_bits_truncate(flags);
	let io_flags = IOFlag::from_bits_truncate(flags);

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let ent = match creation_flags.contains(CreationFlag::O_CREAT) {
		true => lookup_or_create(
			path,
			creation_flags,
			Permission::from_bits_truncate(perm),
			current,
		),
		false => lookup_entry_follow(&path, current),
	}?;

	if creation_flags.contains(CreationFlag::O_TRUNC) {
		ent.clone()
			.downcast_file()
			.and_then(|file| file.truncate(0, current))?;
	}

	let file = ent.open(io_flags, access_flags, current)?;

	let fd = current
		.get_user_ext()
		.expect("must be user process")
		.lock_fd_table()
		.alloc_fd(file)
		.ok_or(Errno::EMFILE)?;

	Ok(fd.index())
}

pub fn sys_creat(path: usize, perm: u32) -> Result<usize, Errno> {
	let flags =
		CreationFlag::O_CREAT.bits() | CreationFlag::O_TRUNC.bits() | AccessFlag::O_RDONLY.bits();

	sys_open(path, flags, perm)
}

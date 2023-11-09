use alloc::sync::Arc;

use crate::fs::path::{Base, Path};
use crate::fs::syscall::AT_FDCWD;
use crate::fs::vfs::{Entry, ROOT_DIR_ENTRY};
use crate::process::fd_table::Fd;
use crate::process::task::Task;
use crate::syscall::errno::Errno;

use super::{VfsDirEntry, VfsEntry};

fn do_lookup_base_entry(
	base_kind: Base,
	base_entry: Arc<VfsDirEntry>,
	task: &Arc<Task>,
) -> Result<Arc<VfsDirEntry>, Errno> {
	let depth = match base_kind {
		Base::RootDir => return ROOT_DIR_ENTRY.lock().clone().ok_or(Errno::ENOENT),
		Base::WorkingDir { to_parent } => to_parent,
	};

	let mut curr = base_entry;

	for _ in 0..depth {
		curr = curr.parent_dir(task)?;
	}

	Ok(curr)
}

fn do_lookup_entry_at(
	base: Arc<VfsDirEntry>,
	path: &Path,
	task: &Arc<Task>,
	follow_mid_symlink: bool,
	follow_last_symlink: bool,
	symlink_depth: usize,
) -> Result<VfsEntry, Errno> {
	if symlink_depth >= 8 {
		return Err(Errno::ELOOP);
	}

	let mut curr = VfsEntry::new_dir(do_lookup_base_entry(path.base(), base, task)?);

	for comp in path.components() {
		use VfsEntry::*;
		curr = match curr {
			SymLink(ref s) => match follow_mid_symlink {
				true => curr.parent_dir(task).and_then(|pdir| {
					do_lookup_entry_at(
						pdir,
						&s.target()?,
						task,
						follow_mid_symlink,
						follow_last_symlink,
						symlink_depth + 1,
					)
					.and_then(|x| x.downcast_dir())
				}),
				false => Err(Errno::ELOOP),
			},
			Dir(d) => Ok(d),
			_ => Err(Errno::ENOTDIR),
		}
		.and_then(|dir| dir.lookup(comp, task))?;
	}

	if follow_last_symlink {
		use VfsEntry::*;
		if let SymLink(s) = curr {
			curr = do_lookup_entry_at(
				s.parent_dir(task)?,
				&s.target()?,
				task,
				follow_mid_symlink,
				follow_last_symlink,
				symlink_depth + 1,
			)?;
		}
	}

	Ok(curr)
}

pub fn lookup_entry(
	base: Arc<VfsDirEntry>,
	path: &Path,
	task: &Arc<Task>,
	follow_mid_symlink: bool,
	follow_last_symlink: bool,
) -> Result<VfsEntry, Errno> {
	do_lookup_entry_at(base, path, task, follow_mid_symlink, follow_last_symlink, 0)
}

pub fn lookup_entry_at_follow(
	base: Arc<VfsDirEntry>,
	path: &Path,
	task: &Arc<Task>,
) -> Result<VfsEntry, Errno> {
	do_lookup_entry_at(base, path, task, true, true, 0)
}

pub fn lookup_entry_follow(path: &Path, task: &Arc<Task>) -> Result<VfsEntry, Errno> {
	let cwd = task
		.get_user_ext()
		.ok_or(Errno::ENOENT)
		.map(|ext| ext.lock_cwd().clone())
		.unwrap_or_else(|_| ROOT_DIR_ENTRY.lock().clone().unwrap());

	lookup_entry_at_follow(cwd, path, task)
}

pub fn lookup_entry_at_nofollow(
	base: Arc<VfsDirEntry>,
	path: &Path,
	task: &Arc<Task>,
) -> Result<VfsEntry, Errno> {
	do_lookup_entry_at(base, path, task, false, false, 0)
}

pub fn lookup_entry_nofollow(path: &Path, task: &Arc<Task>) -> Result<VfsEntry, Errno> {
	let cwd = task
		.get_user_ext()
		.ok_or(Errno::ENOENT)
		.map(|ext| ext.lock_cwd().clone())
		.unwrap_or_else(|_| ROOT_DIR_ENTRY.lock().clone().unwrap());

	lookup_entry_at_nofollow(cwd, path, task)
}

pub fn lookup_entry_at_follow_except_last(
	base: Arc<VfsDirEntry>,
	path: &Path,
	task: &Arc<Task>,
) -> Result<VfsEntry, Errno> {
	do_lookup_entry_at(base, path, task, true, false, 0)
}

pub fn lookup_entry_follow_except_last(path: &Path, task: &Arc<Task>) -> Result<VfsEntry, Errno> {
	let cwd = task
		.get_user_ext()
		.ok_or(Errno::ENOENT)
		.map(|ext| ext.lock_cwd().clone())
		.unwrap_or_else(|_| ROOT_DIR_ENTRY.lock().clone().unwrap());

	lookup_entry_at_follow_except_last(cwd, path, task)
}

pub fn lookup_entry_by_dirfd(dirfd: isize, task: &Arc<Task>) -> Result<VfsEntry, Errno> {
	let user_ext = task.get_user_ext().expect("must be user process");
	match dirfd {
		AT_FDCWD => Ok(VfsEntry::new_dir(user_ext.lock_cwd().clone())),
		x => user_ext
			.lock_fd_table()
			.get_file(Fd::from(x as usize).ok_or(Errno::EBADF)?)
			.ok_or(Errno::ENOENT)
			.and_then(|f| f.as_entry().ok_or(Errno::ENOENT)),
	}
}

pub fn lookup_entry_by_dirfd_path(
	dirfd: isize,
	path: &Path,
	task: &Arc<Task>,
	follow_last_link: bool,
) -> Result<VfsEntry, Errno> {
	let base = lookup_entry_by_dirfd(dirfd, task).and_then(|x| x.downcast_dir())?;

	lookup_entry(base, &path, task, true, follow_last_link)
}

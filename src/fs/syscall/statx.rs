use core::mem::{align_of, size_of};

use alloc::sync::Arc;

use crate::fs::path::Path;
use crate::fs::vfs::{lookup_entry, Entry, Statx, VfsEntry};
use crate::mm::user::verify::{verify_buffer_mut, verify_path};
use crate::process::fd_table::Fd;
use crate::process::task::{Task, CURRENT};
use crate::syscall::errno::Errno;

use super::{AT_EMPTY_PATH, AT_FDCWD, AT_SYMLINK_NOFOLLOW};

fn verify_stat_buf(stat_buf: usize, task: &Arc<Task>) -> Result<&'_ mut Statx, Errno> {
	if stat_buf % align_of::<Statx>() != 0 {
		return Err(Errno::EFAULT);
	}

	let raw_buf = verify_buffer_mut(stat_buf, size_of::<Statx>(), task)?;

	Ok(unsafe { &mut *raw_buf.as_mut_ptr().cast::<Statx>() })
}

fn lookup_entry_by_dirfd(dirfd: isize, task: &Arc<Task>) -> Result<VfsEntry, Errno> {
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

fn lookup_entry_by_dirfd_path(
	dirfd: isize,
	path: &Path,
	task: &Arc<Task>,
	follow_last_link: bool,
) -> Result<VfsEntry, Errno> {
	let base = lookup_entry_by_dirfd(dirfd, task).and_then(|x| x.downcast_dir())?;

	lookup_entry(base, &path, task, true, follow_last_link)
}

pub fn sys_statx(
	dirfd: isize,
	path: usize,
	flags: usize,
	_mask: usize,
	stat_buf: usize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_stat_buf(stat_buf, current)?;

	let entry = match verify_path(path, current) {
		Ok(path) => lookup_entry_by_dirfd_path(
			dirfd,
			&Path::new(path),
			current,
			(flags & AT_SYMLINK_NOFOLLOW) == 0,
		),
		Err(Errno::EINVAL) if (flags & AT_EMPTY_PATH) == AT_EMPTY_PATH => {
			lookup_entry_by_dirfd(dirfd, current)
		}
		Err(e) => Err(e),
	}?;

	let stat = entry.statx()?;

	*buf = stat;

	Ok(0)
}

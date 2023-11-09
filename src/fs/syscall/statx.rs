use core::mem::{align_of, size_of};

use alloc::sync::Arc;

use crate::fs::path::Path;
use crate::fs::vfs::{lookup_entry_by_dirfd, lookup_entry_by_dirfd_path, Entry, Statx};
use crate::mm::user::verify::{verify_buffer_mut, verify_path};
use crate::process::task::{Task, CURRENT};
use crate::syscall::errno::Errno;

use super::{AT_EMPTY_PATH, AT_SYMLINK_NOFOLLOW};

fn verify_stat_buf(stat_buf: usize, task: &Arc<Task>) -> Result<&'_ mut Statx, Errno> {
	if stat_buf % align_of::<Statx>() != 0 {
		return Err(Errno::EFAULT);
	}

	let raw_buf = verify_buffer_mut(stat_buf, size_of::<Statx>(), task)?;

	Ok(unsafe { &mut *raw_buf.as_mut_ptr().cast::<Statx>() })
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

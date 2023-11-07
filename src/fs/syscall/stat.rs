use core::mem::{align_of, size_of};

use alloc::sync::Arc;

use crate::fs::path::Path;
use crate::fs::vfs::{lookup_entry, Entry, Statx};
use crate::mm::user::verify::{verify_buffer_mut, verify_path};
use crate::pr_warn;
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

pub fn sys_statx(
	dirfd: isize,
	path: usize,
	flags: usize,
	_mask: usize,
	stat_buf: usize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_stat_buf(stat_buf, current)?;

	let path = match verify_path(path, current) {
		Ok(x) => Ok(Path::new(x)),
		Err(Errno::EINVAL) if (flags & AT_EMPTY_PATH) == AT_EMPTY_PATH => Ok(Path::new(b"")),
		Err(x) => Err(x),
	}?;

	let user_ext = current.get_user_ext().expect("must be user process");
	let base = match dirfd {
		AT_FDCWD => Ok(user_ext.lock_cwd().clone()),
		x => user_ext
			.lock_fd_table()
			.get_file(Fd::from(x as usize).ok_or(Errno::EBADF)?)
			.ok_or(Errno::ENOENT)
			.and_then(|f| f.as_entry().ok_or(Errno::ENOENT))
			.and_then(|x| x.downcast_dir()),
	}?;

	let entry = lookup_entry(
		base,
		&path,
		current,
		true,
		(flags & AT_SYMLINK_NOFOLLOW) == 0,
	)?;

	let stat = entry.stat()?;

	pr_warn!("STATX: {:#?}", stat);

	*buf = stat;

	Ok(0)
}

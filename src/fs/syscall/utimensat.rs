use crate::{
	fs::{
		path::Path,
		vfs::{lookup_entry_by_dirfd, lookup_entry_by_dirfd_path},
	},
	mm::user::verify::verify_path,
	process::task::CURRENT,
	syscall::errno::Errno,
};

use super::AT_SYMLINK_NOFOLLOW;

pub fn sys_utimensat(
	dirfd: isize,
	pathname: usize,
	_timespec: usize,
	flags: usize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let follow_last_link = (flags & AT_SYMLINK_NOFOLLOW) == 0;
	let _entry = match verify_path(pathname, current) {
		Ok(path) => lookup_entry_by_dirfd_path(dirfd, &Path::new(path), current, follow_last_link),
		Err(Errno::EINVAL) => lookup_entry_by_dirfd(dirfd, current),
		Err(e) => Err(e),
	}?;

	// TODO: entry.utime(...)

	Ok(0)
}

use crate::{
	fs::{path::Path, vfs::lookup_entry_follow},
	mm::user::verify::verify_path,
	process::task::CURRENT,
	syscall::errno::Errno,
};

pub fn sys_truncate(path: usize, length: isize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let entry = lookup_entry_follow(&path, current).and_then(|x| x.downcast_file())?;

	entry.truncate(length, current).map(|_| 0)
}

use crate::{process::task::CURRENT, syscall::errno::Errno};

use super::{read::get_file, utils::verify_buffer_mut};

pub fn sys_getdents(fd: isize, buf: usize, len: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_buffer_mut(buf, len, current)?;

	get_file(fd)?.getdents(buf)
}

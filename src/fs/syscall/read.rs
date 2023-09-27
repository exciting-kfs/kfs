use crate::mm::user::verify::verify_buffer_mut;
use crate::process::task::CURRENT;
use crate::syscall::errno::Errno;

use super::get_file;

pub fn sys_read(fd: isize, buf: usize, len: usize) -> Result<usize, Errno> {
	if len == 0 {
		return Ok(0);
	}

	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_buffer_mut(buf, len, current)?;

	get_file(fd)?.read(buf)
}

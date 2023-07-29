use core::slice::from_raw_parts;

use crate::interrupt::syscall::errno::Errno;

use super::read::get_file;

// TODO copy from user
pub fn sys_write(fd: isize, buf: *const u8, len: usize) -> Result<usize, Errno> {
	let file = get_file(fd)?;
	let buf = unsafe { from_raw_parts(buf, len) };
	file.ops.write(&file, buf)
}

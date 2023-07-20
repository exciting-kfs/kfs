use core::slice::from_raw_parts;

use alloc::sync::Arc;

use crate::{file::File, interrupt::syscall::errno::Errno};

use super::read::get_file;

// TODO copy from user
pub fn sys_write(fd: isize, buf: *const u8, len: isize) -> Result<usize, Errno> {
	fn write(file: &mut Arc<File>, buf: &[u8]) -> usize {
		file.ops.write(buf)
	}

	if len < 0 {
		return Err(Errno::EINVAL);
	}

	let len = len as usize;
	let mut file = get_file(fd)?;
	let mut count = 0;

	// block
	while count < len {
		let buf = unsafe { from_raw_parts(buf.offset(count as isize), len - count) };
		count += write(&mut file, buf);
	}

	Ok(len)
}

use crate::{
	fs::vfs::Whence, mm::user::verify::verify_ptr_mut, process::task::CURRENT,
	syscall::errno::Errno,
};

use super::get_file;

const SEEK_SET: isize = 0;
const SEEK_CUR: isize = 1;
const SEEK_END: isize = 2;

pub fn sys_lseek(fd: isize, offset: isize, raw_whence: isize) -> Result<usize, Errno> {
	let whence = match raw_whence {
		SEEK_SET => Whence::Begin,
		SEEK_CUR => Whence::Current,
		SEEK_END => Whence::End,
		_ => return Err(Errno::EINVAL),
	};

	get_file(fd)?.lseek(offset, whence)
}

pub fn sys_llseek(
	fd: isize,
	h_offset: isize,
	l_offset: isize,
	result: usize,
	raw_whence: isize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let p = verify_ptr_mut::<i64>(result, current).unwrap();
	let offset = (((h_offset as u64) << 32) | (l_offset as u64)) as i64;

	let ret = sys_lseek(fd, offset as isize, raw_whence);

	if let Ok(x) = ret {
		*p = x as i64;
		return Ok(0);
	}

	ret
}

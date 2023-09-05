use crate::{fs::vfs::Whence, syscall::errno::Errno};

use super::read::get_file;

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

use crate::syscall::errno::Errno;

use super::get_file;

pub fn sys_ioctl(fd: isize, request: usize, argp: usize) -> Result<usize, Errno> {
	get_file(fd)?.ioctl(request, argp)
}

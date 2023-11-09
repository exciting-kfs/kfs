use alloc::boxed::Box;

use crate::{
	fs::vfs::{VfsHandle, Whence},
	mm::{constant::PAGE_SIZE, user::verify::verify_ptr_mut},
	process::{fd_table::Fd, task::CURRENT},
};

use super::errno::Errno;

pub fn sys_sendfile(
	out_fd: isize,
	in_fd: isize,
	offset_ptr: usize,
	count: usize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let (dst, src) = {
		let fd_table = current.user_ext_ok_or(Errno::EPERM)?.lock_fd_table();

		let dst = Fd::from(out_fd as usize)
			.and_then(|fd| fd_table.get_file(fd))
			.ok_or(Errno::EBADF)?;
		let src = Fd::from(in_fd as usize)
			.and_then(|fd| fd_table.get_file(fd))
			.ok_or(Errno::EBADF)?;
		(dst, src)
	};

	if let Ok(offset) = verify_ptr_mut::<isize>(offset_ptr, current) {
		let prev_off = src.lseek(0, Whence::Current)?;
		src.lseek(*offset, Whence::Begin)?;

		let ret = __sendfile(src.clone(), dst, count)?;

		src.lseek(prev_off as isize, Whence::Begin)?;
		*offset += ret as isize;
		Ok(ret)
	} else {
		__sendfile(src, dst, count)
	}
}

fn __sendfile(src: VfsHandle, dst: VfsHandle, count: usize) -> Result<usize, Errno> {
	let mut buf = unsafe { Box::new_uninit_slice(PAGE_SIZE).assume_init() };

	let mut sum = 0;
	let mut r_len = src.read(&mut buf)?;

	while sum < count && r_len != 0 {
		let mut w_len = 0;

		while w_len < r_len {
			w_len += dst.write(&buf[w_len..r_len])?;
		}

		sum += r_len;
		r_len = src.read(&mut buf)?;
	}
	Ok(sum)
}

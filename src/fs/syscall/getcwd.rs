use crate::{
	fs::vfs::Entry, mm::user::verify::verify_buffer_mut, process::task::CURRENT,
	syscall::errno::Errno,
};

pub fn sys_getcwd(buf_ptr: usize, len: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_buffer_mut(buf_ptr, len, current)?;

	let path_buf = current
		.get_user_ext()
		.expect("must be user task")
		.lock_cwd()
		.get_abs_path()?
		.to_buffer();

	if buf.len() < path_buf.len() + 1 {
		return Err(Errno::ERANGE);
	}

	buf[0..path_buf.len()].copy_from_slice(&path_buf);
	buf[path_buf.len()] = b'\0';

	Ok(buf_ptr)
}

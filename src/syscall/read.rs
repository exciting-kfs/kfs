use core::{arch::asm, slice::from_raw_parts_mut};

use alloc::sync::Arc;
use kfs_macro::context;

use crate::{file::File, process::task::CURRENT, register};

use super::errno::Errno;

pub fn read(fd: isize, buf: *mut u8, len: isize) -> isize {
	unsafe {
		asm!("int 0x80", in("eax") 0x0, in("ebx") fd, in("ecx") buf, in("edx") len, options(nostack))
	};
	register!("eax") as isize
}

#[context(irq_disabled)]
pub(super) fn get_file(fd: isize) -> Result<Arc<File>, Errno> {
	if fd < 0 {
		return Err(Errno::EBADF);
	}
	let fd = fd as usize;
	let task = unsafe { CURRENT.get_mut() };

	let mut fd_tb = task.fd_table.lock();
	let ret = (fd_tb[fd].as_mut()).ok_or(Errno::EBADF)?;
	Ok(ret.clone())
}

// TODO copy to user.
pub fn sys_read(fd: isize, buf: *mut u8, len: isize) -> Result<usize, Errno> {
	#[context(irq_disabled)]
	fn read(file: &mut Arc<File>, buf: &mut [u8]) -> usize {
		file.ops.read(buf)
	}

	if len < 0 {
		return Err(Errno::EINVAL);
	}

	let len = len as usize;
	let mut file = get_file(fd)?;
	let mut count = 0;

	// block
	while count < len {
		let buf = unsafe { from_raw_parts_mut(buf.offset(count as isize), len - count) };
		count += read(&mut file, buf);
	}

	Ok(len)
}

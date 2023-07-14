use core::{arch::asm, slice::from_raw_parts};

use alloc::sync::Arc;
use kfs_macro::context;

use crate::{file::File, register};

use super::{errno::Errno, read::get_file};

pub fn write(fd: isize, buf: *const u8, len: isize) -> isize {
	unsafe {
		asm!("int 0x80", in("eax") 0x01, in("ebx") fd, in("ecx") buf, in("edx") len, options(nostack))
	};

	register!("eax") as isize
}

// TODO copy from user
pub fn sys_write(fd: isize, buf: *const u8, len: isize) -> Result<usize, Errno> {
	#[context(irq_disabled)]
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

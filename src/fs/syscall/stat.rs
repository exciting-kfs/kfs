use core::mem::{align_of, size_of};

use alloc::sync::Arc;

use crate::fs::path::Path;
use crate::fs::vfs::{lookup_entry_follow, RawStat};
use crate::process::task::{Task, CURRENT};
use crate::syscall::errno::Errno;

use super::utils::{verify_buffer_mut, verify_path};

fn verify_stat_buf(stat_buf: usize, task: &Arc<Task>) -> Result<&'_ mut RawStat, Errno> {
	if stat_buf % align_of::<RawStat>() != 0 {
		return Err(Errno::EFAULT);
	}

	let raw_buf = verify_buffer_mut(stat_buf, size_of::<RawStat>(), task)?;

	Ok(unsafe { &mut *raw_buf.as_mut_ptr().cast::<RawStat>() })
}

pub fn sys_fstat(_fd: isize, _stat_buf: usize) -> Result<usize, Errno> {
	todo!();
}

pub fn sys_stat(path: usize, stat_buf: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_stat_buf(stat_buf, current)?;

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let entry = lookup_entry_follow(&path, current)?;

	let stat = entry.stat()?;

	*buf = stat;

	Ok(0)
}

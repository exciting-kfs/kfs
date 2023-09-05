use core::slice::{from_raw_parts, from_raw_parts_mut};

use alloc::sync::Arc;

use crate::config::PATH_MAX;
use crate::mm::user::vma::{AreaFlag, UserAddressSpace};
use crate::process::task::Task;
use crate::syscall::errno::Errno;

fn query_max_readable_len(start: usize, vma: &UserAddressSpace) -> usize {
	let mut curr = start;

	while let Some(area) = vma.find_area(curr) {
		if !area.flags.contains(AreaFlag::Readable) {
			break;
		}

		if PATH_MAX <= area.end - start {
			return PATH_MAX;
		}

		curr = area.end;
	}

	curr - start
}

fn user_strlen(path: usize, vma: &UserAddressSpace) -> Result<usize, Errno> {
	let max_len = query_max_readable_len(path, vma);

	let length = (0..max_len)
		.map(|i| (path + i) as *const u8)
		.position(|x| unsafe { *x } == 0)
		.ok_or(Errno::EFAULT)?;

	if length == 0 {
		return Err(Errno::EINVAL);
	}

	Ok(length)
}

pub fn verify_path(path: usize, task: &Arc<Task>) -> Result<&'_ [u8], Errno> {
	let memory = task
		.get_user_ext()
		.expect("must be user process")
		.lock_memory();

	let vma = memory.get_vma();

	let length = user_strlen(path, vma)?;

	Ok(unsafe { from_raw_parts(path as *const u8, length) })
}

fn verify_region(
	buf_ptr: usize,
	len: usize,
	task: &Arc<Task>,
	flags: AreaFlag,
) -> Result<(), Errno> {
	let memory = task
		.get_user_ext()
		.expect("must be user process")
		.lock_memory();
	if !memory.query_flags_range(buf_ptr, len, flags) {
		return Err(Errno::EFAULT);
	}

	Ok(())
}

pub fn verify_buffer_mut(
	buf_ptr: usize,
	len: usize,
	task: &Arc<Task>,
) -> Result<&'_ mut [u8], Errno> {
	verify_region(buf_ptr, len, task, AreaFlag::Writable)?;

	Ok(unsafe { from_raw_parts_mut(buf_ptr as *mut u8, len) })
}

pub fn verify_buffer(buf_ptr: usize, len: usize, task: &Arc<Task>) -> Result<&'_ [u8], Errno> {
	verify_region(buf_ptr, len, task, AreaFlag::Readable)?;

	Ok(unsafe { from_raw_parts(buf_ptr as *const u8, len) })
}

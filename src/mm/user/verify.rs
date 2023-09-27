use core::mem::size_of;
use core::slice::{from_raw_parts, from_raw_parts_mut};

use alloc::sync::Arc;

use crate::config::PATH_MAX;
use crate::mm::user::vma::{AreaFlag, UserAddressSpace};
use crate::process::task::Task;
use crate::syscall::errno::Errno;

fn query_max_readable_len(start: usize, vma: &UserAddressSpace, limit: usize) -> usize {
	let mut curr = start;

	while let Some(area) = vma.find_area(curr) {
		if !area.flags.contains(AreaFlag::Readable) {
			break;
		}

		if limit <= area.end - start {
			return limit;
		}

		curr = area.end;
	}

	curr - start
}

fn user_strlen(path: usize, vma: &UserAddressSpace, limit: usize) -> Result<usize, Errno> {
	let max_len = query_max_readable_len(path, vma, limit);

	let length = (0..max_len)
		.map(|i| (path + i) as *const u8)
		.position(|x| unsafe { *x } == 0)
		.ok_or(Errno::EFAULT)?;

	Ok(length)
}

pub fn verify_path(path: usize, task: &Arc<Task>) -> Result<&'_ [u8], Errno> {
	let memory = task
		.get_user_ext()
		.expect("must be user process")
		.lock_memory();

	let vma = memory.get_vma();

	let length = user_strlen(path, vma, PATH_MAX)?;

	if length == 0 {
		return Err(Errno::EINVAL);
	}

	Ok(unsafe { from_raw_parts(path as *const u8, length) })
}

pub fn verify_string(string: usize, task: &Arc<Task>, limit: usize) -> Result<&'_ [u8], Errno> {
	let memory = task
		.get_user_ext()
		.expect("must be user process")
		.lock_memory();

	let vma = memory.get_vma();

	let length = user_strlen(string, vma, limit)?;

	Ok(unsafe { from_raw_parts(string as *const u8, length) })
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

/// T must be PDO type (usize, i32, u32 ... etc)
pub fn verify_ptr<T>(ptr: usize, task: &Arc<Task>) -> Result<&'_ T, Errno> {
	verify_region(ptr, size_of::<T>(), task, AreaFlag::Readable)?;

	Ok(unsafe { &*(ptr as *const T) })
}

/// T must be PDO type (usize, i32, u32 ... etc)
pub fn verify_ptr_mut<T>(ptr: usize, task: &Arc<Task>) -> Result<&'_ mut T, Errno> {
	verify_region(ptr, size_of::<T>(), task, AreaFlag::Writable)?;

	Ok(unsafe { &mut *(ptr as *mut T) })
}

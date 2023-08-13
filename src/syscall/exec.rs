use core::{ffi::CStr, mem, slice};

use crate::{
	config::{USER_CODE_BASE, USTACK_BASE, USTACK_PAGES},
	interrupt::InterruptFrame,
	mm::user::{memory::Memory, vma::AreaFlag},
	process::task::CURRENT,
	syscall::errno::Errno,
	user_bin,
};

const PATH_MAX: usize = 128;

/// execute new user binary
/// do not call from kernel thread!!
pub fn sys_exec(frame: *mut InterruptFrame, raw_binary_name_ptr: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let mut memory = current
		.get_user_ext()
		.expect("not user process")
		.lock_memory();

	let area = memory
		.get_vma()
		.find_area(raw_binary_name_ptr)
		.ok_or_else(|| Errno::EFAULT)?;

	if !area.flags.contains(AreaFlag::Readable) {
		return Err(Errno::EFAULT);
	}

	let max_safe_len = (area.end - raw_binary_name_ptr).min(PATH_MAX);

	let bytes = unsafe { slice::from_raw_parts(raw_binary_name_ptr as *const u8, max_safe_len) };

	let binary_name = CStr::from_bytes_until_nul(bytes).map_err(|_| Errno::EFAULT)?;
	let binary_name = binary_name.to_str().map_err(|_| Errno::ENOENT)?;

	let code = user_bin::get_user_bin(binary_name).ok_or_else(|| Errno::ENOENT)?;

	let new_memory =
		Memory::new(USTACK_BASE, USTACK_PAGES, USER_CODE_BASE, code).map_err(|_| Errno::ENOMEM)?;

	new_memory.pick_up();

	mem::drop(mem::replace(&mut *memory, new_memory));

	unsafe {
		frame.copy_from_nonoverlapping(&InterruptFrame::new_user(USER_CODE_BASE, USTACK_BASE), 1)
	};

	Ok(0)
}

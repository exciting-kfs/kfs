use core::{ffi::CStr, mem, ptr::addr_of_mut};

use crate::{
	config::{USER_CODE_BASE, USTACK_BASE, USTACK_PAGES},
	interrupt::InterruptFrame,
	mm::user::{memory::Memory, verify::verify_string},
	process::task::CURRENT,
	syscall::errno::Errno,
	user_bin,
};

const PATH_MAX: usize = 128;

/// execute new user binary
/// do not call from kernel thread!!
pub fn sys_execve(
	frame: *mut InterruptFrame,
	raw_binary_name_ptr: usize,
	argv: usize,
	envp: usize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	verify_string(raw_binary_name_ptr, current, PATH_MAX)?;
	let binary_name = unsafe { CStr::from_ptr(raw_binary_name_ptr as *const i8) };
	let binary_name = binary_name.to_str().map_err(|_| Errno::ENOENT)?;

	let code = user_bin::get_user_bin(binary_name).ok_or_else(|| Errno::ENOENT)?;

	let mut new_memory =
		Memory::new(USTACK_BASE, USTACK_PAGES, USER_CODE_BASE, code).map_err(|_| Errno::ENOMEM)?;

	let (argv_begin, argv_count) = new_memory.push_string_array(argv, current)?;
	let (envp_begin, _) = new_memory.push_string_array(envp, current)?;

	new_memory.pick_up();

	let mut memory = current
		.get_user_ext()
		.expect("must be user process")
		.lock_memory();

	mem::drop(mem::replace(&mut *memory, new_memory));

	unsafe {
		frame.copy_from_nonoverlapping(&InterruptFrame::new_user(USER_CODE_BASE, USTACK_BASE), 1);
		addr_of_mut!((*frame).edi).write(argv_begin);
		addr_of_mut!((*frame).edx).write(argv_count);
		addr_of_mut!((*frame).esi).write(envp_begin);
	};

	Ok(0)
}

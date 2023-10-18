use crate::{
	config::PATH_MAX, elf::kobject::cleanup_kernel_module, mm::user::verify::verify_string,
	process::task::CURRENT, syscall::errno::Errno,
};

pub fn sys_cleanup_module(name_ptr: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	if !current.is_privileged() {
		return Err(Errno::EPERM);
	}

	let module_name = verify_string(name_ptr, current, PATH_MAX)?;

	cleanup_kernel_module(module_name).map(|_| 0)
}

use crate::{process::task::CURRENT, syscall::errno::Errno};

pub fn sys_brk(new_end: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let mut memory = current
		.get_user_ext()
		.expect("must be user process")
		.lock_memory();

	if new_end == 0 {
		return Ok(memory.get_data_end());
	}

	memory.brk(new_end)
}

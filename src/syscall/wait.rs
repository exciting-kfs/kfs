use core::mem::size_of;

use crate::{
	mm::user::vma::AreaFlag,
	process::{
		relation::{Pgid, Pid},
		task::CURRENT,
	},
	scheduler::context::yield_now,
	syscall::errno::Errno,
};

#[derive(Clone, Copy)]
pub enum Who {
	Any,
	Pid(Pid),
	Pgid(Pgid),
}

mod wait_option {
	pub const WNOHANG: usize = 1 << 0;
	pub const WUNTRACED: usize = 1 << 1; // not implemented.
	pub const IMPLEMENTED_MASK: usize = WNOHANG;
}

pub fn sys_waitpid(cpid: isize, stat_loc: *mut isize, option: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	if !current
		.get_user_ext()
		.expect("must be user process")
		.lock_memory()
		.query_flags_range(stat_loc as usize, size_of::<isize>(), AreaFlag::Writable)
	{
		return Err(Errno::EFAULT);
	}

	// unknown option
	if (option & wait_option::IMPLEMENTED_MASK) != option {
		return Err(Errno::EINVAL);
	}

	let who = match cpid {
		-1 => Who::Any,
		0 => Who::Pgid(current.get_pgid()),
		x if x < 0 => Who::Pgid(Pgid::from_raw(-x as usize)),
		x if x > 0 => Who::Pid(Pid::from_raw(x as usize)),
		_ => unreachable!("obviously unreachable..."),
	};

	let non_block = (option & wait_option::WNOHANG) != 0;

	let ret = loop {
		let result = current.waitpid(who);
		if let Ok(z) = result {
			unsafe { stat_loc.write(z.exit_status.as_raw() as isize) };
		}

		let ret = result.map(|z| z.pid.as_raw());
		if non_block || ret.is_ok() {
			break ret;
		}

		yield_now();
	};

	ret
}

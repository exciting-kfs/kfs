use crate::{
	pr_debug,
	process::{signal::sig_num::SigNum, task::CURRENT},
	scheduler::context::yield_now,
};

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ExitStatus {
	raw: usize,
}

impl ExitStatus {
	pub fn new_signaled(termsig: u8) -> Self {
		Self {
			raw: (termsig & 0x7f) as usize,
		}
	}

	pub fn new_exited(status: u8) -> Self {
		Self {
			raw: (status as usize) << 8,
		}
	}

	pub fn as_raw(&self) -> usize {
		self.raw
	}
}

pub fn sys_exit(status: usize) -> ! {
	let current = unsafe { CURRENT.get_mut() };
	current.exit(ExitStatus::new_exited(status as u8));

	yield_now();
	unreachable!("cannot scheduled after sys_exit");
}

pub fn exit_with_signal(sig: SigNum) -> ! {
	let current = unsafe { CURRENT.get_mut() };

	pr_debug!("{:?} exit with SIG{:?}", current.get_pid(), sig);
	current.exit(ExitStatus::new_signaled(sig as usize as u8));

	yield_now();
	unreachable!("cannot scheduled after exit_with_signal");
}

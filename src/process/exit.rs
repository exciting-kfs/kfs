use crate::signal::sig_num::SigNum;

use super::{context::yield_now, task::CURRENT};

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ExitStatus {
	raw: usize,
}

#[repr(usize)]
pub enum ExitFlag {
	Signaled = 0x0100_0000,
	Stopped = 0x0200_0000,
	Exited = 0x0300_0000,
	CoreDumped = 0x0400_0000,
}

impl ExitStatus {
	pub fn new(flag: ExitFlag, status: u8) -> Self {
		Self {
			raw: flag as usize | status as usize,
		}
	}

	pub fn as_raw(&self) -> usize {
		self.raw
	}
}

pub fn sys_exit(status: usize) -> ! {
	// context_switch(InContext::IrqDisabled);
	let current = unsafe { CURRENT.get_mut() };
	current.exit(ExitStatus::new(ExitFlag::Exited, status as u8));

	yield_now();
	unreachable!("cannot scheduled after sys_exit");
}

pub fn exit_with_signal(sig: SigNum) -> ! {
	// context_switch(InContext::IrqDisabled);
	let current = unsafe { CURRENT.get_mut() };

	current.exit(ExitStatus::new(ExitFlag::Signaled, sig as usize as u8));

	yield_now();
	unreachable!("cannot scheduled after exit_with_signal");
}

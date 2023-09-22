use crate::interrupt::in_interrupt_context;
use crate::process::signal::poll_signal_queue;
use crate::scheduler::context::yield_now;
use crate::syscall::errno::Errno;

use super::RawSpinLock;
use super::TryLockFail;

#[derive(Debug)]
pub struct LocalSpinLock {
	raw: RawSpinLock,
}

unsafe impl Sync for LocalSpinLock {}

impl LocalSpinLock {
	pub const fn new() -> Self {
		LocalSpinLock {
			raw: RawSpinLock::new(),
		}
	}

	pub fn lock_check_signal(&self) -> Result<(), Errno> {
		check_precondition();

		while let Err(_) = self.raw.try_lock() {
			unsafe { poll_signal_queue()? };
			yield_now();
		}

		Ok(())
	}

	pub fn lock(&self) {
		check_precondition();

		while let Err(_) = self.raw.try_lock() {
			yield_now();
		}
	}

	pub fn try_lock(&self) -> Result<(), TryLockFail> {
		check_precondition();

		self.raw.try_lock()
	}

	pub fn unlock(&self) {
		self.raw.unlock();
	}
}

fn check_precondition() {
	debug_assert!(!in_interrupt_context(), "msg")
}

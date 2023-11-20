use core::sync::atomic::{AtomicBool, Ordering};

use super::{Error, Workable};

pub struct WorkOnce {
	func: fn() -> (),
	atomic: AtomicBool,
}

impl WorkOnce {
	pub const fn new(func: fn() -> ()) -> Self {
		Self {
			func,
			atomic: AtomicBool::new(false),
		}
	}

	pub fn schedulable(&self) -> bool {
		self.atomic
			.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
			.is_ok()
	}
}

impl Workable for WorkOnce {
	fn work(&self) -> Result<(), Error> {
		(self.func)();
		self.atomic.store(false, Ordering::Release);
		Ok(())
	}
}

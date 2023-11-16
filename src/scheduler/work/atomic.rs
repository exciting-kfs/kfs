use core::sync::atomic::{AtomicBool, Ordering};

use alloc::sync::Arc;

use super::{Error, Workable, FAST_WORK_POOL};

pub struct AtomicWork {
	func: fn() -> (),
	atomic: AtomicBool,
}

impl AtomicWork {
	pub const fn new(func: fn() -> ()) -> Self {
		Self {
			func,
			atomic: AtomicBool::new(false),
		}
	}

	pub fn schedule(this: &Arc<Self>) {
		if let Ok(_) =
			this.atomic
				.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
		{
			FAST_WORK_POOL.lock().push_back(this.clone());
		}
	}
}

impl Workable for AtomicWork {
	fn work(&self) -> Result<(), Error> {
		(self.func)();
		self.atomic.store(false, Ordering::Release);
		Ok(())
	}
}

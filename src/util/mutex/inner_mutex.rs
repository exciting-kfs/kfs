use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

#[derive(Debug)]
pub struct InnerMutex {
	lock_atomic: AtomicBool,
}

unsafe impl Sync for InnerMutex {}

impl InnerMutex {
	pub const fn new() -> Self {
		InnerMutex {
			lock_atomic: AtomicBool::new(false),
		}
	}

	pub fn lock(&self) {
		while let Err(_) =
			self.lock_atomic
				.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
		{}
	}

	pub fn unlock(&self) {
		self.lock_atomic.store(false, Ordering::Relaxed);
	}
}

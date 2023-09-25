mod global_spin_lock;
mod local_spin_lock;

pub use global_spin_lock::{get_lock_depth, GlobalSpinLock};
pub use local_spin_lock::LocalSpinLock;

use core::arch::asm;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use super::TryLockFail;

#[derive(Debug)]
pub struct RawSpinLock {
	is_locked: AtomicBool,
}

unsafe impl Sync for RawSpinLock {}

impl RawSpinLock {
	pub const fn new() -> Self {
		RawSpinLock {
			is_locked: AtomicBool::new(false),
		}
	}

	pub fn lock(&self) {
		use Ordering::*;
		while let Err(_) = self
			.is_locked
			.compare_exchange(false, true, Acquire, Acquire)
		{
			unsafe { asm!("pause") };
		}
	}

	pub fn try_lock(&self) -> Result<(), TryLockFail> {
		use Ordering::*;
		match self
			.is_locked
			.compare_exchange(false, true, Acquire, Acquire)
		{
			Ok(_) => Ok(()),
			Err(_) => Err(TryLockFail),
		}
	}

	pub fn unlock(&self) {
		self.is_locked.store(false, Ordering::Release);
	}
}

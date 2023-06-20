use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use crate::interrupt::irq_disable;
use crate::interrupt::irq_enable;
use crate::interrupt::pop_irq_stack;
use crate::interrupt::push_irq_stack;

use super::TryLockFail;

#[derive(Debug)]
pub struct SpinLock {
	lock_atomic: AtomicBool,
}

unsafe impl Sync for SpinLock {}

impl SpinLock {
	pub const fn new() -> Self {
		SpinLock {
			lock_atomic: AtomicBool::new(false),
		}
	}

	pub fn lock(&self) {
		while let Err(_) =
			self.lock_atomic
				.compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
		{
			unsafe { core::arch::asm!("pause") };
		}
	}

	pub fn lock_irq(&self) {
		irq_disable();
		self.lock();
	}

	pub fn lock_irq_save(&self) {
		push_irq_stack();
		self.lock();
	}

	pub fn try_lock(&self) -> Result<(), TryLockFail> {
		match self
			.lock_atomic
			.compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
		{
			Ok(_) => Ok(()),
			Err(_) => Err(TryLockFail),
		}
	}

	pub fn unlock(&self) {
		self.lock_atomic.store(false, Ordering::Release);
	}

	pub fn unlock_irq(&self) {
		self.unlock();
		irq_enable();
	}

	pub fn unlock_irq_save(&self) {
		self.unlock();
		pop_irq_stack();
	}
}

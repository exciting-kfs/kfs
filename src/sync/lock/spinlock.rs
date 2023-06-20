use core::cell::UnsafeCell;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use crate::interrupt::check_interrupt_flag;
use crate::interrupt::irq_disable;
use crate::interrupt::irq_enable;

use super::TryLockFail;

#[derive(Debug)]
pub struct SpinLock {
	lock_atomic: AtomicBool,
	iflag: UnsafeCell<bool>,
}

unsafe impl Sync for SpinLock {}

impl SpinLock {
	pub const fn new() -> Self {
		SpinLock {
			lock_atomic: AtomicBool::new(false),
			iflag: UnsafeCell::new(false),
		}
	}

	pub fn lock(&self) {
		while let Err(_) =
			self.lock_atomic
				.compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
		{
			unsafe { core::arch::asm!("pause") };
		}

		irq_disable();
		unsafe { (*self.iflag.get()) = check_interrupt_flag() }
	}

	pub fn try_lock(&self) -> Result<(), TryLockFail> {
		match self
			.lock_atomic
			.compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
		{
			Ok(_) => unsafe {
				irq_disable();
				(*self.iflag.get()) = check_interrupt_flag();
				Ok(())
			},
			Err(_) => Err(TryLockFail),
		}
	}

	pub fn unlock(&self) {
		if unsafe { *self.iflag.get() } {
			irq_enable();
		}
		self.lock_atomic.store(false, Ordering::Release);
	}
}

use core::arch::asm;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use crate::interrupt::in_interrupt_context;
use crate::interrupt::irq_disable;
use crate::interrupt::irq_enable;
use crate::sync::cpu_local::CpuLocal;

use super::TryLockFail;

/// ## Caution
///
/// - In `SpinLock` implementation, we can't use `printk` function.
#[derive(Debug)]
pub struct SpinLock {
	lock_atomic: AtomicBool,
}

unsafe impl Sync for SpinLock {}

/// lock state is not cloned.
impl Clone for SpinLock {
	fn clone(&self) -> Self {
		Self::new()
	}
}

static LOCK_DEPTH: CpuLocal<usize> = CpuLocal::new(0);

pub fn get_lock_depth() -> usize {
	unsafe { *LOCK_DEPTH.get_mut() }
}

fn inc_lock_depth() {
	unsafe { *LOCK_DEPTH.get_mut() += 1 };
}

fn dec_lock_depth() {
	unsafe { *LOCK_DEPTH.get_mut() -= 1 };
}

impl SpinLock {
	pub const fn new() -> Self {
		SpinLock {
			lock_atomic: AtomicBool::new(false),
		}
	}

	pub fn lock(&self) {
		irq_disable();
		while let Err(_) =
			self.lock_atomic
				.compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
		{
			unsafe { asm!("pause") };
		}
		inc_lock_depth();
	}

	pub fn try_lock(&self) -> Result<(), TryLockFail> {
		irq_disable();
		let result =
			self.lock_atomic
				.compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire);

		if result.is_ok() {
			inc_lock_depth();

			Ok(())
		} else {
			irq_enable();

			Err(TryLockFail)
		}
	}

	pub fn unlock(&self) {
		self.lock_atomic.store(false, Ordering::Release);
		dec_lock_depth();

		if get_lock_depth() == 0 && !in_interrupt_context() {
			irq_enable();
		}
	}
}

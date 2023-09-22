use crate::interrupt::in_interrupt_context;
use crate::interrupt::irq_disable;
use crate::interrupt::irq_enable;
use crate::sync::cpu_local::CpuLocal;

use super::RawSpinLock;
use super::TryLockFail;

#[derive(Debug)]
pub struct GlobalSpinLock {
	raw: RawSpinLock,
}

unsafe impl Sync for GlobalSpinLock {}

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

impl GlobalSpinLock {
	pub const fn new() -> Self {
		GlobalSpinLock {
			raw: RawSpinLock::new(),
		}
	}

	pub fn lock(&self) {
		irq_disable();
		self.raw.lock();
		inc_lock_depth();
	}

	pub fn try_lock(&self) -> Result<(), TryLockFail> {
		irq_disable();

		let result = self.raw.try_lock();
		match result {
			Ok(_) => inc_lock_depth(),
			Err(_) => irq_enable(),
		};

		result
	}

	pub fn unlock(&self) {
		self.raw.unlock();

		dec_lock_depth();
		if get_lock_depth() == 0 && !in_interrupt_context() {
			irq_enable();
		}
	}
}

mod exception;
mod interrupt_frame;

pub mod idt;

use core::arch::asm;

pub use interrupt_frame::InterruptFrame;

use crate::sync::{cpu_local::CpuLocal, spinlock::get_lock_depth};

#[inline(always)]
pub fn irq_enable() {
	unsafe { asm!("sti") };
}

#[inline(always)]
pub fn irq_disable() {
	unsafe { asm!("cli") };
}

pub fn is_sti() -> bool {
	let flag_mask = 1 << 9;
	let mut eflags: usize;
	unsafe {
		asm!(
			"pushfd",
			"pop eax",
			out("eax") eflags
		)
	};

	eflags & flag_mask == flag_mask
}

static IN_INTERRUPT: CpuLocal<bool> = CpuLocal::new(true);

pub struct InterruptGuard(());

impl Drop for InterruptGuard {
	fn drop(&mut self) {
		unsafe { *IN_INTERRUPT.get_mut() = false };
	}
}

pub fn enter_interrupt_context() -> InterruptGuard {
	unsafe { *IN_INTERRUPT.get_mut() = true };
	InterruptGuard(())
}

pub unsafe extern "C" fn leave_interrupt_context() {
	*IN_INTERRUPT.get_mut() = false;
}

pub fn in_interrupt_context() -> bool {
	unsafe { *IN_INTERRUPT.get_ref() }
}

pub struct InterruptBackup(bool);

impl Drop for InterruptBackup {
	fn drop(&mut self) {
		assert_eq!(get_lock_depth(), 0);

		unsafe { *IN_INTERRUPT.get_mut() = self.0 };

		if !self.0 {
			irq_enable();
		}
	}
}

pub fn save_interrupt_context() -> InterruptBackup {
	irq_disable();
	assert_eq!(get_lock_depth(), 0);

	let backup = InterruptBackup(unsafe { *IN_INTERRUPT.get_ref() });

	unsafe { *IN_INTERRUPT.get_mut() = true };

	backup
}

pub fn kthread_init() {
	unsafe { *IN_INTERRUPT.get_mut() = false };
	irq_enable();
}

#[cfg(disable)]
mod tests {
	use super::*;
	use kfs_macro::ktest;

	#[ktest(dev)]
	fn test() {
		unsafe { core::arch::asm!("sti") };
		assert!(is_sti());
		unsafe { core::arch::asm!("cli") };
		assert!(!is_sti());
	}
}

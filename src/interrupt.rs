mod exception;
mod hw;
mod interrupt_frame;

pub mod apic;
pub mod idt;
pub mod syscall;

use core::arch::asm;

pub use hw::apic_timer::jiffies;
pub use interrupt_frame::InterruptFrame;

use crate::sync::cpu_local::CpuLocal;

#[inline(always)]
pub fn irq_enable() {
	unsafe { asm!("sti") };
}

#[inline(always)]
pub fn irq_disable() {
	unsafe { asm!("cli") };
}

pub fn get_interrupt_flag() -> bool {
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

pub fn in_interrupt_context() -> bool {
	unsafe { *IN_INTERRUPT.get_mut() }
}

#[cfg(disable)]
mod tests {
	use super::*;
	use kfs_macro::ktest;

	#[ktest(dev)]
	fn test() {
		unsafe { core::arch::asm!("sti") };
		assert!(get_interrupt_flag());
		unsafe { core::arch::asm!("cli") };
		assert!(!get_interrupt_flag());
	}
}

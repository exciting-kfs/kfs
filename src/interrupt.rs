mod exception;
mod hw;
mod interrupt_frame;

pub mod apic;
pub mod idt;
pub mod privilege_level;
pub mod tasklet;

use core::arch::asm;

pub use apic::MSR_APIC_BASE;
pub use hw::apic_timer::jiffies;
pub use interrupt_frame::InterruptFrame;

pub fn irq_enable() {
	unsafe { asm!("sti") };
}

pub fn irq_disable() {
	unsafe { asm!("cli") };
}

fn get_interrupt_flag() -> bool {
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

mod exception;
mod hw;
mod interrupt_frame;

pub mod apic;
pub mod idt;
pub mod idte;
pub mod privilege_level;

use core::arch::asm;

pub use apic::local_id as lapic_id;
pub use apic::LAPIC_PBASE;
pub use apic::MSR_APIC_BASE;

pub use interrupt_frame::InterruptFrame;

use crate::config::NR_CPUS;

pub fn irq_enable() {
	unsafe { asm!("sti") };
}

pub fn irq_disable() {
	unsafe { asm!("cli") };
}

#[must_use]
pub fn irq_save() -> bool {
	let iflag = get_interrupt_flag();
	irq_disable();

	iflag
}

pub fn irq_restore(iflag: bool) {
	if iflag {
		irq_enable();
	} else {
		irq_disable();
	}
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

#[derive(Clone, Copy)]
struct IrqCount {
	nmi: u8,
	hw: u8,
	sw: u8,
}

impl IrqCount {
	const fn new() -> Self {
		IrqCount {
			nmi: 0,
			hw: 0,
			sw: 0,
		}
	}

	fn inc_sw(&mut self) -> Option<u8> {
		self.sw.checked_add(1)
	}

	fn dec_sw(&mut self) -> Option<u8> {
		self.sw.checked_sub(1)
	}

	fn inc_hw(&mut self) -> Option<u8> {
		self.hw.checked_add(1)
	}

	fn dec_hw(&mut self) -> Option<u8> {
		self.hw.checked_sub(1)
	}

	fn inc_nmi(&mut self) -> Option<u8> {
		self.nmi.checked_add(1)
	}

	fn dec_nmi(&mut self) -> Option<u8> {
		self.nmi.checked_sub(1)
	}
}

static mut IRQ_COUNT: [IrqCount; NR_CPUS] = [IrqCount::new(); NR_CPUS];

pub fn in_interrupt() -> bool {
	in_hw_irq() || in_sw_irq() || in_nmi()
}

pub fn in_sw_irq() -> bool {
	unsafe { IRQ_COUNT[lapic_id()] }.sw > 0
}

pub fn in_hw_irq() -> bool {
	unsafe { IRQ_COUNT[lapic_id()] }.hw > 0
}

pub fn in_nmi() -> bool {
	unsafe { IRQ_COUNT[lapic_id()] }.nmi > 0
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

mod exception;
mod hw;
mod interrupt_frame;

pub mod apic;
pub mod idt;
pub mod idte;
pub mod privilege_level;

pub use apic::local_id as lapic_id;
pub use apic::LAPIC_PBASE;
pub use apic::MSR_APIC_BASE;

pub use interrupt_frame::InterruptFrame;

use crate::config::NR_CPUS;

pub fn irq_enable() {
	unsafe { core::arch::asm!("sti") };
}

pub fn irq_disable() {
	unsafe { core::arch::asm!("cli") };
}

fn get_interrupt_flag() -> bool {
	let flag_mask = 1 << 9;
	let mut eflags: usize;
	unsafe {
		core::arch::asm!(
			"pushfd",
			"pop eax",
			out("eax") eflags
		)
	};

	eflags & flag_mask == flag_mask
}

#[derive(Clone, Copy)]
struct IrqStack {
	sti: bool,
	cli: usize,
}

impl IrqStack {
	const fn new() -> Self {
		Self { sti: false, cli: 0 }
	}

	fn push(&mut self, iflag: bool) {
		if self.sti && iflag {
			panic!("irq stack push");
		}

		match iflag {
			true => self.sti = true,
			false => self.cli += 1,
		}
	}

	fn pop(&mut self) -> bool {
		if self.cli == 0 && !self.sti {
			panic!("irq stack pop");
		}

		match self.cli == 0 {
			true => self.sti = false,
			false => self.cli -= 1,
		}

		self.sti
	}
}

static mut IRQ_STACK: [IrqStack; NR_CPUS] = [IrqStack::new(); NR_CPUS];

pub fn irq_stack_save() {
	let iflag = get_interrupt_flag();

	unsafe { IRQ_STACK[lapic_id()].push(iflag) };
	irq_disable();
}

pub fn irq_stack_restore() {
	if unsafe { IRQ_STACK[lapic_id()].pop() } {
		irq_enable();
	} else {
		irq_disable();
	}
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

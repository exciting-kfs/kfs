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

pub fn irq_enable() {
	unsafe { core::arch::asm!("sti") };
}

pub fn irq_disable() {
	unsafe { core::arch::asm!("cli") };
}

pub fn check_interrupt_flag() -> bool {
	let flag_mask = 1 << 9;
	let mut eflags: usize;
	unsafe { core::arch::asm!("pushfd", "popfd {}", out(reg) eflags) }

	eflags & flag_mask == flag_mask
}

use crate::interrupt::{enter_interrupt_context, irq_enable};

use super::exit::sys_exit;

/// Re-enable IRQ and execute thread routine
pub extern "C" fn kthread_entry(routine: extern "C" fn(usize) -> usize, arg: usize) {
	kthread_init();
	let ret = routine(arg);
	sys_exit(ret);
}

pub fn kthread_init() {
	drop(enter_interrupt_context());
	irq_enable();
}

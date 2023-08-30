use crate::interrupt::irq_enable;

use super::exit::sys_exit;

/// Re-enable IRQ and execute thread routine
pub extern "C" fn kthread_entry(routine: extern "C" fn(usize) -> usize, arg: usize) {
	irq_enable();
	let ret = routine(arg);
	sys_exit(ret);
}

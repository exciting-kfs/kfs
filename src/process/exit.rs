use crate::interrupt::irq_disable;

use super::{
	context::yield_now,
	family::{ExitStatus, TASK_TREE},
	task::{State, CURRENT},
};

pub fn sys_exit(status: usize) -> ! {
	irq_disable();
	let current = unsafe { CURRENT.get_mut() };

	*current.lock_state() = State::Exited;

	if !current.is_kernel() {
		TASK_TREE
			.lock()
			.exit_task(current.get_pid(), ExitStatus { raw: status });
	}

	yield_now();

	unreachable!("cannot scheduled after sys_exit");
}

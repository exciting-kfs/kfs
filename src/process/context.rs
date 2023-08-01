use core::mem;

use alloc::sync::Arc;
use kfs_macro::interrupt_handler;

use super::task::{State, Task, CURRENT, TASK_QUEUE};

use crate::x86::CPU_TASK_STATE;

/// yield control from current task to next task
///  call flow: yield_now -> switch_stack -> switch_task_finish
#[interrupt_handler]
pub fn yield_now() {
	let next = {
		let mut task_q = TASK_QUEUE.lock();

		match task_q.pop_front() {
			Some(x) => x,
			None => return,
		}
	};

	// safety: IRQ is disabled.
	let curr = unsafe { CURRENT.get_mut() }.clone();

	let curr_task = Arc::into_raw(curr);
	let next_task = Arc::into_raw(next);

	unsafe { switch_stack(curr_task, next_task) };
}

extern "fastcall" {
	/// switch stack and call switch_task_finish
	///
	/// defined at asm/interrupt.S
	#[allow(improper_ctypes)]
	pub fn switch_stack(curr: *const Task, next: *const Task);
}

#[no_mangle]
pub unsafe extern "fastcall" fn switch_task_finish(curr: *const Task, next: *const Task) {
	let curr = Arc::from_raw(curr);
	let next = Arc::from_raw(next);

	CPU_TASK_STATE
		.get_mut()
		.change_kernel_stack(next.kstack_base());

	{
		let state_lock = curr.lock_state();
		if *state_lock == State::Running {
			mem::drop(state_lock);
			TASK_QUEUE.lock().push_back(curr);
		};
	};

	if let Some(user) = next.get_user_ext() {
		user.lock_memory().pick_up();
	}

	let _ = mem::replace(CURRENT.get_mut(), next);
}

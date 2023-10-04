use core::mem;

use alloc::sync::Arc;

use crate::{
	interrupt::save_interrupt_context,
	process::task::{State, Task, CURRENT},
	scheduler::{
		preempt::{get_preempt_count, preemptable},
		TASK_QUEUE,
	},
	x86::CPU_TASK_STATE,
};

/// yield control from current task to next task
///  call flow: yield_now -> switch_stack -> switch_task_finish
pub fn yield_now() {
	debug_assert!(
		preemptable(),
		"please drop `AtomicOps` before this call {}",
		get_preempt_count()
	);
	let _ctx = save_interrupt_context();

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

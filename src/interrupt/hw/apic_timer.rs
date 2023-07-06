use crate::{
	interrupt::{apic::end_of_interrupt, InterruptFrame},
	process::{
		context::switch_stack,
		task::{CURRENT, TASK_QUEUE},
	},
};

#[no_mangle]
pub unsafe extern "C" fn handle_timer_impl(_frame: InterruptFrame) {
	end_of_interrupt();

	let task_q = unsafe { TASK_QUEUE.lock_manual() };

	// safety: this function always called through interrupt gate.
	// so IRQ is disabled.
	let current = unsafe { CURRENT.get_mut() };

	let next = match task_q.pop_front() {
		Some(x) => x,
		None => return,
	};
	task_q.push_back(next.clone());

	let prev_stack = unsafe { current.get_manual().esp_mut() };
	let next_stack = unsafe { next.get_manual().esp_mut() };

	core::mem::drop(core::mem::replace(current, next));

	switch_stack(prev_stack, next_stack);

	unsafe { TASK_QUEUE.unlock_manual() }
}

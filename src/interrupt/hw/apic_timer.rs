use crate::{
	interrupt::{apic::end_of_interrupt, InterruptFrame},
	process::{
		context::switch_stack,
		task::{CURRENT, TASK_QUEUE},
	},
};

#[no_mangle]
pub unsafe extern "C" fn handle_timer_impl(_frame: &InterruptFrame) {
	end_of_interrupt();

	let mut task_q = TASK_QUEUE.lock();
	let mut current = CURRENT.get_mut();

	let prev = &mut *current;
	let next = task_q.pop_front().unwrap();

	task_q.push_back(next);

	let next = task_q.back_mut().unwrap();

	core::mem::swap(prev, next);

	switch_stack(next.esp_mut(), prev.esp_mut());
}

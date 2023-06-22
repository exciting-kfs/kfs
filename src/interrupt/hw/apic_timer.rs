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

	let next = task_q.pop_front().unwrap();
	let prev = CURRENT.replace(next);
	task_q.push_back(prev);

	let prev = task_q.back_mut().unwrap();
	let next = &mut *current;

	switch_stack(prev.esp_mut(), next.esp_mut());
}

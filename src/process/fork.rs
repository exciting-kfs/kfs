use crate::interrupt::InterruptFrame;

use super::task::{CURRENT, TASK_QUEUE};

// do not call from kernel context
pub fn sys_fork(frame: *mut InterruptFrame) {
	let current = unsafe { CURRENT.get_mut() };

	let forked = current.clone_for_fork(frame).expect("OOM");

	TASK_QUEUE.lock().push_back(forked);
}

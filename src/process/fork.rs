use crate::interrupt::InterruptFrame;

use super::task::{CURRENT, TASK_QUEUE};

// do not call from kernel context
pub fn sys_fork(frame: *mut InterruptFrame) {
	let current = unsafe { CURRENT.get_mut() };

	if let Ok(forked) = current.clone_for_fork(frame) {
		TASK_QUEUE.lock().push_back(forked);
	}
}

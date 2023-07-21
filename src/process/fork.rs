use kfs_macro::context;

use crate::interrupt::{syscall::errno::Errno, InterruptFrame};

use super::task::{CURRENT, TASK_QUEUE};

// do not call from kernel context
#[context(irq_disabled)]
pub fn sys_fork(frame: *const InterruptFrame) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	if let Ok(forked) = current.clone_for_fork(frame) {
		let pid = forked.get_pid().as_raw();
		TASK_QUEUE.lock().push_back(forked);
		Ok(pid)
	} else {
		Err(Errno::ENOMEM)
	}
}

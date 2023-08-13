use crate::{
	interrupt::InterruptFrame, process::task::CURRENT, scheduler::TASK_QUEUE, syscall::errno::Errno,
};

pub fn sys_fork(frame: *const InterruptFrame) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	if let Ok(forked) = current.clone_for_fork(frame) {
		let pid = forked.get_pid().as_raw();
		let mut tq = TASK_QUEUE.lock();

		tq.push_back(forked);

		Ok(pid)
	} else {
		Err(Errno::ENOMEM)
	}
}

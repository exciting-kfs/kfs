pub mod sleep;
pub mod work;

use alloc::sync::Arc;

use crate::{
	process::{
		context::yield_now,
		task::{Task, TASK_QUEUE},
	},
	syscall::errno::Errno,
};

pub type SyncTask = Arc<Task>;

pub fn schedule_first(task: SyncTask) {
	let mut q = TASK_QUEUE.lock();
	q.push_front(task);
}

pub fn schedule_last(task: SyncTask) {
	let mut q = TASK_QUEUE.lock();
	q.push_back(task);
}

pub fn sys_sched_yield() -> Result<usize, Errno> {
	yield_now();
	Ok(0)
}

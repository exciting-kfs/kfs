use alloc::sync::Arc;

use crate::process::task::{Task, TASK_QUEUE};

pub mod work;

pub type SyncTask = Arc<Task>;

pub fn schedule_first(task: SyncTask) {
	let mut q = TASK_QUEUE.lock();
	q.push_front(task);
}

pub fn schedule_last(task: SyncTask) {
	let mut q = TASK_QUEUE.lock();
	q.push_back(task);
}

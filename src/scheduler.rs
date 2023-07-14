use alloc::sync::Arc;
use kfs_macro::context;

use crate::process::task::{Task, TASK_QUEUE};

pub mod work;

pub type SyncTask = Arc<Task>;

#[context(irq_disabled)]
pub fn schedule_first(task: SyncTask) {
	TASK_QUEUE.lock().push_front(task);
}

#[context(irq_disabled)]
pub fn schedule_last(task: SyncTask) {
	TASK_QUEUE.lock().push_back(task);
}

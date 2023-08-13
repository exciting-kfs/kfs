pub mod context;
pub mod sleep;
pub mod work;

use alloc::{collections::LinkedList, sync::Arc};

use crate::{process::task::Task, sync::locked::Locked, syscall::errno::Errno};

use self::context::yield_now;

pub static TASK_QUEUE: Locked<LinkedList<Arc<Task>>> = Locked::new(LinkedList::new());

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

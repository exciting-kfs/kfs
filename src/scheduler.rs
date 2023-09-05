pub mod context;
pub mod sleep;
pub mod work;

use alloc::{collections::LinkedList, sync::Arc};

use crate::{process::task::Task, sync::locked::Locked, syscall::errno::Errno};

use self::context::yield_now;

static TASK_QUEUE: Locked<LinkedList<Arc<Task>>> = Locked::new(LinkedList::new());

pub fn schedule_first(task: Arc<Task>) {
	let mut q = TASK_QUEUE.lock();
	q.push_front(task);
}

pub fn schedule_last(task: Arc<Task>) {
	let mut q = TASK_QUEUE.lock();
	q.push_back(task);
}

pub fn sys_sched_yield() -> Result<usize, Errno> {
	yield_now();
	Ok(0)
}

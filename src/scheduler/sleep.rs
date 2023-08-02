use alloc::sync::Arc;

use crate::{
	process::{
		context::yield_now,
		relation::job::session::Session,
		task::{State, Task, CURRENT, PROCESS_TREE, TASK_QUEUE},
	},
	sync::locked::Locked,
};

pub fn sleep_and_yield() {
	let current = unsafe { CURRENT.get_mut() };
	*current.lock_state() = State::Sleeping;

	// pr_debug!("pid[{}] sleep!", current.get_pid().as_raw());

	yield_now();
}

pub fn wake_up(task: &Arc<Task>) {
	let mut state_lock = task.lock_state();
	if *state_lock == State::Sleeping {
		// pr_debug!("pid[{}] wake up!", task.get_pid().as_raw());
		*state_lock = State::Running;
		TASK_QUEUE.lock().push_back(task.clone());
	}
}

pub fn wake_up_foreground(sess: &Arc<Locked<Session>>) {
	if let Some(fg) = sess.lock().foreground() {
		for pid in fg.lock().members() {
			let ptree = PROCESS_TREE.lock();
			let task = ptree.get(pid).expect("task in process group.");
			wake_up(task);
		}
	}
}

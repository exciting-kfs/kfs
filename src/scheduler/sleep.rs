use alloc::sync::{Arc, Weak};

use crate::{
	process::{
		context::yield_now,
		relation::session::Session,
		task::{State, Task, CURRENT, TASK_QUEUE},
	},
	sync::locked::Locked,
};

pub fn sleep_and_yield(state: State) {
	debug_assert!(state == State::Sleeping || state == State::DeepSleep);

	let current = unsafe { CURRENT.get_mut() };
	*current.lock_state() = state;

	// pr_debug!("pid[{}] sleep!", current.get_pid().as_raw());

	yield_now();
}

pub fn wake_up(task: &Arc<Task>, state: State) {
	debug_assert!(state == State::Sleeping || state == State::DeepSleep);

	let mut state_lock = task.lock_state();
	if *state_lock == state || *state_lock == State::Sleeping {
		// pr_debug!("{:?} wake up!", task.get_pid());
		*state_lock = State::Running;
		drop(state_lock);
		TASK_QUEUE.lock().push_back(task.clone());
	}
}

pub fn wake_up_foreground(sess: &Weak<Locked<Session>>, state: State) -> Option<()> {
	let sess = sess.upgrade()?;
	let sess_lock = sess.lock();
	let fg = sess_lock.foreground()?.upgrade()?;

	for (_, weak) in fg.lock_members().iter() {
		if let Some(task) = weak.upgrade() {
			wake_up(&task, state);
		}
	}
	None
}

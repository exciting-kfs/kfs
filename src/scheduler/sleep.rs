use alloc::sync::{Arc, Weak};

use crate::{
	process::{
		process_tree::PROCESS_TREE,
		relation::{session::Session, Pid},
		task::{State, Task, CURRENT},
	},
	scheduler::{context::yield_now, schedule_last},
	sync::Locked,
	syscall::errno::Errno,
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
		schedule_last(task.clone());
	}
}

pub fn wake_up_pid(pid: Pid, state: State) -> Result<(), Errno> {
	let tree = PROCESS_TREE.lock();
	let task = tree.get(&pid).ok_or(Errno::ESRCH)?;
	wake_up(task, state);
	Ok(())
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

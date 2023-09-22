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

use super::preempt::AtomicOps;

pub enum Sleep {
	Light,
	Deep,
}

pub fn sleep_and_yield(sleep: Sleep) {
	let current = unsafe { CURRENT.get_mut() };
	*current.lock_state() = match sleep {
		Sleep::Deep => State::DeepSleep,
		Sleep::Light => State::Sleeping,
	};

	// pr_debug!("pid[{}] sleep!", current.get_pid().as_raw());

	yield_now();
}

pub fn sleep_and_yield_atomic(sleep: Sleep, atomic: AtomicOps) {
	let current = unsafe { CURRENT.get_mut() };
	*current.lock_state() = match sleep {
		Sleep::Deep => State::DeepSleep,
		Sleep::Light => State::Sleeping,
	};

	drop(atomic);
	yield_now();
}

pub fn sleep_and_yield_atomic_optional(sleep: Sleep, atomic: Option<AtomicOps>) {
	match atomic {
		Some(a) => sleep_and_yield_atomic(sleep, a),
		None => sleep_and_yield(sleep),
	}
}

pub fn wake_up_deep_sleep(task: &Arc<Task>) {
	let mut state_lock = task.lock_state();
	if *state_lock == State::DeepSleep || *state_lock == State::Sleeping {
		*state_lock = State::Running;
		drop(state_lock);
		schedule_last(task.clone());
	}
}

pub fn wake_up_sleep(task: &Arc<Task>) {
	let mut state_lock = task.lock_state();
	if *state_lock == State::Sleeping {
		*state_lock = State::Running;
		drop(state_lock);
		schedule_last(task.clone());
	}
}

pub fn wake_up(task: &Arc<Task>, state: State) {
	match state {
		State::DeepSleep => wake_up_deep_sleep(task),
		State::Sleeping => wake_up_sleep(task),
		_ => {}
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

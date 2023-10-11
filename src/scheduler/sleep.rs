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
	trace_feature,
};

use super::preempt::AtomicOps;

#[derive(Copy, Clone, Debug)]
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

	yield_now();
}

pub fn sleep_and_yield_atomic(sleep: Sleep, atomic: AtomicOps) {
	let current = unsafe { CURRENT.get_mut() };
	*current.lock_state() = match sleep {
		Sleep::Deep => State::DeepSleep,
		Sleep::Light => State::Sleeping,
	};

	trace_feature!("sleep_atomic", "sleep: {:?}", current.get_pid());

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

pub fn wake_up(task: &Arc<Task>, sleep: Sleep) {
	match sleep {
		Sleep::Deep => wake_up_deep_sleep(task),
		Sleep::Light => wake_up_sleep(task),
	}
}

pub fn wake_up_pid(pid: Pid, sleep: Sleep) -> Result<(), Errno> {
	let tree = PROCESS_TREE.lock();
	let task = tree.get(&pid).ok_or(Errno::ESRCH)?;
	wake_up(task, sleep);
	Ok(())
}

pub fn wake_up_foreground(sess: &Weak<Locked<Session>>, sleep: Sleep) -> Option<()> {
	let sess = sess.upgrade()?;
	let sess_lock = sess.lock();
	let fg = sess_lock.foreground()?.upgrade()?;

	for (_, weak) in fg.lock_members().iter() {
		if let Some(task) = weak.upgrade() {
			wake_up(&task, sleep);
		}
	}
	None
}

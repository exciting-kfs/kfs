use super::{
	context::yield_now,
	task::{State, CURRENT},
};

pub fn sys_exit(_status: usize) {
	let current = unsafe { CURRENT.get_mut() };

	*current.lock_state() = State::Exited;
	yield_now();
}

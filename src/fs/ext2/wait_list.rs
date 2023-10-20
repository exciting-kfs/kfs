use alloc::{
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{
	process::task::{Task, CURRENT},
	scheduler::sleep::wake_up_deep_sleep,
	trace_feature,
};

#[derive(Debug)]
pub struct WaitList {
	list: Vec<Weak<Task>>,
}

impl WaitList {
	pub fn new() -> Self {
		Self { list: Vec::new() }
	}

	pub fn register(&mut self) {
		let current = unsafe { CURRENT.get_ref() };
		let w = Arc::downgrade(current);

		self.list.push(w);
	}
}

impl Drop for WaitList {
	fn drop(&mut self) {
		trace_feature!(
			"ext2-waitlist",
			"wake up: {}",
			self.list
				.iter()
				.filter_map(|w| w.upgrade().map(|t| t.get_pid()))
				.collect::<Vec<_>>()
		);

		self.list.iter().for_each(|w| {
			if let Some(task) = w.upgrade() {
				wake_up_deep_sleep(&task)
			}
		})
	}
}

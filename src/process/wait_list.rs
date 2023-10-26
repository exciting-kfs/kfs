use core::mem::take;

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

	pub fn wake_up_all(&mut self) {
		let list = take(&mut self.list);

		trace_feature!(
			"waitlist",
			"wake up: {}",
			list.iter()
				.filter_map(|w| w.upgrade().map(|t| t.get_pid()))
				.collect::<Vec<_>>()
		);

		list.into_iter().for_each(|w| {
			if let Some(task) = w.upgrade() {
				wake_up_deep_sleep(&task)
			}
		})
	}
}

impl Drop for WaitList {
	fn drop(&mut self) {
		self.wake_up_all();
	}
}

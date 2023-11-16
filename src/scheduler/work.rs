pub mod atomic;
pub mod default;

use core::alloc::AllocError;
use core::mem;

use alloc::sync::Arc;
use alloc::{boxed::Box, collections::LinkedList};

use crate::mm::oom::wake_up_oom_handler;
use crate::process::task::{State, Task};
use crate::sync::Locked;
use crate::trace_feature;

use self::default::DefaultWork;

use super::context::yield_now;
use super::schedule_last;

pub trait Workable {
	fn work(&self) -> Result<(), Error>;
}

pub enum Error {
	Alloc,
	Retry,
	Next(Box<dyn Workable>),
}

static FAST_WORK_POOL: Locked<LinkedList<Arc<dyn Workable>>> = Locked::new(LinkedList::new());
static SLOW_WORK_POOL: Locked<LinkedList<Box<dyn Workable>>> = Locked::new(LinkedList::new());

pub fn schedule_worker<ArgType: 'static>(
	func: fn(usize) -> (),
	arg: Box<ArgType>,
) -> Result<(), AllocError> {
	let arg = Box::into_raw(arg) as usize;

	let task = Task::new_kernel(func as usize, arg)?;
	schedule_last(task);

	Ok(())
}

pub fn schedule_slow_work<ArgType: 'static>(
	func: fn(&mut ArgType) -> Result<(), Error>,
	arg: ArgType,
) {
	let arg = Box::new(arg);
	let work = Box::new(DefaultWork::new(func, arg));
	let mut pool = SLOW_WORK_POOL.lock();
	pool.push_back(work);
}

pub fn fast_worker(_: usize) {
	fn take_work() -> Option<Arc<dyn Workable>> {
		let mut pool = FAST_WORK_POOL.lock();
		pool.pop_front()
	}

	while let Some(w) = take_work() {
		let _ = w.work();
	}
}

pub fn slow_worker(_: usize) {
	loop {
		fast_worker(0);
		let works = {
			let mut pool = SLOW_WORK_POOL.lock();
			mem::take(&mut *pool)
		};

		trace_feature!(
			"worker",
			"WORKER: work start! mili: {}",
			crate::driver::hpet::get_timestamp_mili() % 1000,
		);

		let len = works.len();

		for work in works {
			if let Err(e) = work.work() {
				match e {
					Error::Alloc => wake_up_oom_handler(),
					Error::Retry => SLOW_WORK_POOL.lock().push_back(work),
					Error::Next(next) => SLOW_WORK_POOL.lock().push_back(next),
				}
			}
		}

		if len == 0 {
			yield_now();
			trace_feature!(
				"worker",
				"WORKER: wake up! mili: {}, fast len: {}, slow len: {}",
				crate::driver::hpet::get_timestamp_mili() % 1000,
				FAST_WORK_POOL.lock().len(),
				SLOW_WORK_POOL.lock().len()
			);
		}
	}
}

pub fn init() -> Result<(), AllocError> {
	let worker = Task::new_kernel(fast_worker as usize, 0)?;
	*worker.lock_state() = State::Exited;

	Ok(())
}

pub mod default;
pub mod once;

use core::alloc::AllocError;
use core::mem;

use alloc::sync::Arc;
use alloc::{boxed::Box, collections::LinkedList};

use crate::mm::oom::wake_up_oom_handler;
use crate::process::task::{State, Task};
use crate::sync::Locked;
use crate::trace_feature;

use self::default::WorkDefault;
use self::once::WorkOnce;

use super::context::yield_now;

pub trait Workable {
	fn work(&self) -> Result<(), Error>;
}

pub enum Work {
	Fast(Arc<dyn Workable>),
	Slow(Arc<dyn Workable>),
}

impl Work {
	pub fn new_default<ArgType: 'static>(
		func: fn(&mut ArgType) -> Result<(), Error>,
		arg: ArgType,
	) -> Self {
		let arg = Box::new(arg);
		let work = Arc::new(WorkDefault::new(func, arg));
		Work::Slow(work)
	}

	pub fn new_once(work: Arc<WorkOnce>) -> Option<Self> {
		if work.schedulable() {
			Some(Work::Fast(work))
		} else {
			None
		}
	}
}

pub enum Error {
	Alloc,
	Retry,
	Next(Arc<dyn Workable>),
}

static FAST_WORK_POOL: Locked<LinkedList<Arc<dyn Workable>>> = Locked::new(LinkedList::new());
static SLOW_WORK_POOL: Locked<LinkedList<Arc<dyn Workable>>> = Locked::new(LinkedList::new());

pub fn schedule_work(work: Work) {
	match work {
		Work::Fast(f) => FAST_WORK_POOL.lock().push_back(f),
		Work::Slow(s) => SLOW_WORK_POOL.lock().push_back(s),
	}
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

use core::mem;
use core::{alloc::AllocError, mem::MaybeUninit};

use alloc::sync::Arc;
use alloc::{boxed::Box, collections::LinkedList};

use crate::mm::oom::wake_up_oom_handler;
use crate::process::task::{State, Task};
use crate::sync::Locked;

use super::context::yield_now;
use super::{schedule_first, schedule_last};

pub struct Work<ArgType> {
	func: fn(&mut ArgType) -> Result<(), Error>,
	arg: Box<ArgType>,
}

impl<ArgType> Work<ArgType> {
	pub fn new(func: fn(&mut ArgType) -> Result<(), Error>, arg: Box<ArgType>) -> Self {
		Self { func, arg }
	}
}

pub trait Workable {
	fn work(&mut self) -> Result<(), Error>;
}

impl<ArgType> Workable for Work<ArgType> {
	fn work(&mut self) -> Result<(), Error> {
		(self.func)(self.arg.as_mut())
	}
}

pub enum Error {
	Alloc,
	Retry,
	Next(Box<dyn Workable>),
}

static FAST_WORK_POOL: Locked<LinkedList<Box<dyn Workable>>> = Locked::new(LinkedList::new());
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
	let work = Box::new(Work::new(func, arg));
	let mut pool = SLOW_WORK_POOL.lock();
	pool.push_back(work);
}

pub fn schedule_fast_work<ArgType: 'static>(
	func: fn(&mut ArgType) -> Result<(), Error>,
	arg: ArgType,
) {
	let arg = Box::new(arg);
	let work = Box::new(Work::new(func, arg));
	let mut pool = FAST_WORK_POOL.lock();
	pool.push_back(work);
}

pub fn fast_worker(_: usize) {
	fn take_work() -> Option<Box<dyn Workable>> {
		let mut pool = FAST_WORK_POOL.lock();
		pool.pop_front()
	}

	while let Some(mut w) = take_work() {
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

		if works.len() == 0 {
			yield_now();
		}

		for mut work in works {
			if let Err(e) = work.work() {
				match e {
					Error::Alloc => wake_up_oom_handler(),
					Error::Retry => SLOW_WORK_POOL.lock().push_back(work),
					Error::Next(next) => SLOW_WORK_POOL.lock().push_back(next),
				}
			}
		}
	}
}

// FIXME
static mut FAST_WORKER: MaybeUninit<Arc<Task>> = MaybeUninit::uninit();

// FIXME
pub fn wakeup_fast_woker() {
	let task = unsafe { FAST_WORKER.assume_init_mut().clone() };

	let mut state = task.lock_state();
	if *state == State::Running {
		return;
	}
	*state = State::Running;
	drop(state);

	schedule_first(task);
}

pub fn init() -> Result<(), AllocError> {
	let worker = Task::new_kernel(fast_worker as usize, 0)?;
	*worker.lock_state() = State::Exited;

	unsafe { FAST_WORKER.write(worker) };
	Ok(())
}

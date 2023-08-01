use core::mem;
use core::{alloc::AllocError, mem::MaybeUninit};

use alloc::{boxed::Box, collections::LinkedList};

use crate::mm::alloc::phys::Atomic;
use crate::process::context::yield_now;
use crate::process::task::{State, Task};
use crate::sync::locked::Locked;

use super::{schedule_first, SyncTask};

pub struct Work<ArgType> {
	func: fn(&mut ArgType),
	arg: Box<ArgType, Atomic>,
}

impl<ArgType> Work<ArgType> {
	pub fn new(func: fn(&mut ArgType), arg: Box<ArgType, Atomic>) -> Self {
		Self { func, arg }
	}
}

pub trait Workable {
	fn work(&mut self);
}

impl<ArgType> Workable for Work<ArgType> {
	fn work(&mut self) {
		(self.func)(self.arg.as_mut())
	}
}

static FAST_WORK_POOL: Locked<LinkedList<Box<dyn Workable, Atomic>>> =
	Locked::new(LinkedList::new());
static SLOW_WORK_POOL: Locked<LinkedList<Box<dyn Workable, Atomic>>> =
	Locked::new(LinkedList::new());
static mut FAST_WORKER: MaybeUninit<SyncTask> = MaybeUninit::uninit();

pub fn schedule_slow_work<ArgType: 'static>(func: fn(&mut ArgType), arg: ArgType) {
	let arg = Box::new_in(arg, Atomic);
	let work = Box::new_in(Work::new(func, arg), Atomic);
	let mut pool = SLOW_WORK_POOL.lock();
	pool.push_back(work);
}

pub fn schedule_fast_work<ArgType: 'static>(func: fn(&mut ArgType), arg: ArgType) {
	let arg = Box::new_in(arg, Atomic);
	let work = Box::new_in(Work::new(func, arg), Atomic);
	let mut pool = FAST_WORK_POOL.lock();
	pool.push_back(work);
}

pub fn fast_worker(_: usize) {
	fn take_work() -> Option<Box<dyn Workable, Atomic>> {
		let mut pool = FAST_WORK_POOL.lock();
		pool.pop_front()
	}

	while let Some(mut w) = take_work() {
		w.work()
	}
}

pub fn slow_worker(_: usize) {
	loop {
		fast_worker(0);
		let works = {
			let mut pool = SLOW_WORK_POOL.lock();
			mem::replace(&mut *pool, LinkedList::new())
		};

		if works.len() == 0 {
			yield_now();
		}

		for mut w in works {
			w.work();
		}
	}
}

// irq_disabled
pub fn wakeup_fast_woker() {
	let task = unsafe { FAST_WORKER.assume_init_mut().clone() };

	{
		let mut state = task.lock_state();
		// already enqueued or running.
		if *state != State::Exited {
			return;
		}
		*state = State::Running;
	}

	schedule_first(task);
}

pub fn init() -> Result<(), AllocError> {
	let worker = Task::new_kernel(fast_worker as usize, 0)?;
	unsafe { FAST_WORKER.write(worker) };
	Ok(())
}

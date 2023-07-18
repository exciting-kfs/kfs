use core::{alloc::AllocError, mem::MaybeUninit};

use alloc::{boxed::Box, collections::LinkedList};

use kfs_macro::context;

use crate::mm::alloc::phys::Atomic;
use crate::process::context::yield_now;
use crate::process::task::{State, Task};
use crate::sync::singleton::Singleton;

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

static FAST_WORK_POOL: Singleton<LinkedList<Box<dyn Workable, Atomic>>> = Singleton::uninit();
static SLOW_WORK_POOL: Singleton<LinkedList<Box<dyn Workable, Atomic>>> = Singleton::uninit();
static mut FAST_WORKER: MaybeUninit<SyncTask> = MaybeUninit::uninit();

#[context(irq_disabled)]
pub fn schedule_slow_work<ArgType: 'static>(func: fn(&mut ArgType), arg: ArgType) {
	let arg = Box::new_in(arg, Atomic);
	let work = Box::new_in(Work::new(func, arg), Atomic);
	SLOW_WORK_POOL.lock().push_back(work);
}

#[context(irq_disabled)]
pub fn schedule_fast_work<ArgType: 'static>(func: fn(&mut ArgType), arg: ArgType) {
	let arg = Box::new_in(arg, Atomic);
	let work = Box::new_in(Work::new(func, arg), Atomic);
	FAST_WORK_POOL.lock().push_back(work);
}

pub fn fast_worker(_: usize) {
	#[context(irq_disabled)]
	fn take_work() -> Option<Box<dyn Workable, Atomic>> {
		FAST_WORK_POOL.lock().pop_front()
	}

	while let Some(mut w) = take_work() {
		(*w).work()
	}
}

pub fn slow_worker(_: usize) {
	loop {
		fast_worker(0);
		let works = SLOW_WORK_POOL.replace(LinkedList::new());

		if works.len() == 0 {
			yield_now();
		}

		for mut w in works {
			(*w).work();
		}
	}
}

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

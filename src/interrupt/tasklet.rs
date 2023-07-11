use alloc::{boxed::Box, collections::LinkedList};
use kfs_macro::context;

use crate::{interrupt::jiffies, sync::cpu_local::CpuLocal};

pub struct Tasklet<ArgType> {
	func: fn(&mut ArgType),
	arg: Box<ArgType>,
}

impl<ArgType> Tasklet<ArgType> {
	pub fn new(func: fn(&mut ArgType), arg: Box<ArgType>) -> Self {
		Self { func, arg }
	}
}

pub trait Workable {
	fn work(&mut self);
}

impl<ArgType> Workable for Tasklet<ArgType> {
	fn work(&mut self) {
		(self.func)(self.arg.as_mut())
	}
}

static TASKLET_POOL: CpuLocal<LinkedList<Box<dyn Workable>>> = CpuLocal::uninit();

#[context(irq_disabled)]
pub fn schedule_tasklet<ArgType: 'static>(tl: Tasklet<ArgType>) {
	unsafe { TASKLET_POOL.get_mut().push_back(Box::new(tl)) };
}

#[context(irq_disabled)]
pub fn reschedule_tasklet(tl: Box<dyn Workable>) {
	unsafe { TASKLET_POOL.get_mut().push_back(tl) };
}

#[context(irq_disabled)]
fn take_tasklet() -> LinkedList<Box<dyn Workable>> {
	unsafe { TASKLET_POOL.replace(LinkedList::new()) }
}

#[context(preempt_disabled)]
pub fn do_tasklet_timeout() {
	const TASKLET_JIFFIES_TIMEOUT: usize = 20;
	let timeout = jiffies() + TASKLET_JIFFIES_TIMEOUT;
	let tasklets = take_tasklet();

	tasklets.into_iter().for_each(|mut tasklet| {
		if jiffies() < timeout {
			tasklet.work();
		} else {
			reschedule_tasklet(tasklet);
		}
	});
}

pub fn do_tasklet_all() {
	let tasklets = take_tasklet();

	tasklets.into_iter().for_each(|mut tasklet| tasklet.work())
}

#[cfg(tasklet)]
mod tests {
	use crate::pr_debug;
	use kfs_macro::ktest;

	use super::*;

	fn func(arg: usize) {
		pr_debug!("tasklet doing: {}", arg);
	}

	#[context(irq_disabled)]
	fn func_irq_disabled(arg: usize) {
		pr_debug!("tasklet doing: {}", arg);
	}

	fn func_struct(arg: usize) {
		let ptr = arg as *mut TestStruct;
		let arg = unsafe { ptr.as_mut() }.unwrap();

		pr_debug!("tasklet doing: {:?}", arg);
	}

	#[derive(Debug)]
	struct TestStruct {
		a: usize,
		b: bool,
	}

	#[ktest(tasklet)]
	fn test() {
		context_switch(InContext::Kernel);

		TASKLET_POOL.init(Vec::new());

		let tl = Tasklet::new(func, 1);
		schedule_tasklet(tl);

		let tl = Tasklet::new(func_irq_disabled, 2);
		schedule_tasklet(tl);

		let a = TestStruct { a: 1, b: false };
		let tl = Tasklet::new(func_struct, &a as *const TestStruct as usize);
		schedule_tasklet(tl);

		do_tasklet_all();
	}
}

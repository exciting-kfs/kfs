use crate::backtrace::kernel_stack_top;

use self::task::{Stack, Task, CURRENT};

pub mod context;
pub mod kthread;
pub mod task;
pub mod user_space;

pub fn init() {
	let kstack = unsafe { Stack::from_raw(kernel_stack_top as usize as *mut _) };

	let idle_task = Task::alloc_new(kstack).unwrap();

	CURRENT.init(idle_task);
}

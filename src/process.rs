pub mod context;
pub mod exec;
pub mod exit;
pub mod fork;
pub mod kstack;
pub mod kthread;
pub mod task;

use self::{
	kstack::Stack,
	task::{Task, CURRENT},
};
use crate::backtrace::kernel_stack_top;

pub fn init() {
	let kstack = unsafe { Stack::from_raw(kernel_stack_top as usize as *mut _) };

	let idle_task = Task::new_kernel_from_raw(kstack);

	CURRENT.init(idle_task);
}

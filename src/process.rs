pub mod context;
pub mod exec;
pub mod exit;
pub mod fd_table;
pub mod fork;
pub mod kstack;
pub mod kthread;
pub mod relation;
pub mod task;
pub mod uid;
pub mod wait;

use core::mem::MaybeUninit;

use self::{
	kstack::Stack,
	relation::Pid,
	task::{Task, CURRENT},
};

use crate::{backtrace::kernel_stack_top, user_bin};
use alloc::sync::Arc;

static mut INIT_TASK: MaybeUninit<Arc<Task>> = MaybeUninit::uninit();
static mut IDLE_TASK: MaybeUninit<Arc<Task>> = MaybeUninit::uninit();

pub fn init() {
	let idle_kstack = unsafe { Stack::from_raw(kernel_stack_top as usize as *mut _) };
	let idle_task = Task::new_kernel_from_raw(Pid::from_raw(0), idle_kstack);
	CURRENT.init(idle_task.clone());
	unsafe { IDLE_TASK.write(idle_task) };

	let init_task = Task::new_init_task(user_bin::INIT).expect("OOM");
	unsafe { INIT_TASK.write(init_task) };
}

pub fn get_idle_task() -> Arc<Task> {
	unsafe { IDLE_TASK.assume_init_ref() }.clone()
}

pub fn get_init_task() -> Arc<Task> {
	unsafe { INIT_TASK.assume_init_ref() }.clone()
}

pub mod exit;
pub mod fd_table;
pub mod gid;
pub mod kstack;
pub mod kthread;
pub mod process_tree;
pub mod relation;
pub mod signal;
pub mod task;
pub mod uid;
pub mod wait_list;

use core::mem::MaybeUninit;

use self::{
	kstack::Stack,
	relation::Pid,
	task::{Task, CURRENT},
};

use crate::{user_bin::get_user_elf, util::backtrace::kernel_stack_top};
use alloc::sync::Arc;

static mut INIT_TASK: MaybeUninit<Arc<Task>> = MaybeUninit::uninit();
static mut IDLE_TASK: MaybeUninit<Arc<Task>> = MaybeUninit::uninit();

pub fn init() {
	let idle_kstack = unsafe { Stack::from_raw(kernel_stack_top as usize as *mut _) };
	let idle_task = Task::new_kernel_from_raw(Pid::allocate(), idle_kstack);
	CURRENT.init(idle_task.clone());
	unsafe { IDLE_TASK.write(idle_task) };

	let init = get_user_elf("init").expect("invalid INIT elf file");
	let init_task = Task::new_init_task(Pid::allocate(), init).expect("OOM");
	unsafe { INIT_TASK.write(init_task) };
}

pub fn get_idle_task() -> Arc<Task> {
	unsafe { IDLE_TASK.assume_init_ref() }.clone()
}

pub fn get_init_task() -> Arc<Task> {
	unsafe { INIT_TASK.assume_init_ref() }.clone()
}

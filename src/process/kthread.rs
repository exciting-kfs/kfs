use alloc::{boxed::Box, sync::Arc};
use core::alloc::AllocError;

use super::task::{Task, TASK_QUEUE};
use crate::{
	interrupt::{irq_enable, irq_stack_restore},
	sync::locked::Locked,
};

extern "C" {
	/// Immediately execute new task created by `kthread_create`
	/// see asm/interrupt.S
	pub fn kthread_exec(esp: *mut usize) -> !;
}

/// Cleanup IRQ mask and locks after new kernel thread started.
unsafe extern "C" fn kthread_exec_cleanup(callback: extern "C" fn(usize) -> !, arg: usize) {
	unsafe { TASK_QUEUE.unlock_manual() };
	irq_enable();

	callback(arg);
}

/// create new kernel thread.
/// after created, stack will be looks like...
///
/// |IDX | DESC                        |
/// | -- | :--:                        |
/// |  0 | EBX (0)                     |
/// |  4 | EDI (0)                     |
/// |  8 | ESI (0)                     |
/// | 12 | EBP (0)                     |
/// | 16 | EIP1 (kthread_exec_cleanup) |
/// | 20 | EIP  for EIP1 (0)           |
/// | 24 | ARG1 for EIP1               |
/// | 28 | ARG2 for EIP1               |
pub fn kthread_create(main: usize, arg: usize) -> Result<Arc<Locked<Task>>, AllocError> {
	let new_task = Task::alloc_new()?;

	{
		let mut task = new_task.lock();

		task.kstack.push(arg).unwrap();
		task.kstack.push(main).unwrap();
		task.kstack.push(0).unwrap();
		task.kstack.push(kthread_exec_cleanup as usize).unwrap();
		task.kstack.push(0).unwrap();
		task.kstack.push(0).unwrap();
		task.kstack.push(0).unwrap();
		task.kstack.push(0).unwrap();
	}

	Ok(new_task)
}

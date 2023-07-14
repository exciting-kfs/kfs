use alloc::sync::Arc;
use core::alloc::AllocError;

use crate::process::task::{yield_now, CURRENT};

use super::{
	context::{context_switch, InContext},
	task::{Stack, State, Task},
};

/// Cleanup IRQ mask and locks after new kernel thread started.
unsafe extern "C" fn kthread_entry(callback: extern "C" fn(usize) -> usize, arg: usize) {
	context_switch(InContext::Kernel);
	let ret = callback(arg);
	sys_exit(ret);
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
/// | 16 | EIP1 (kthread_entry)        |
/// | 20 | EIP  for EIP1 (0)           |
/// | 24 | ARG1 for EIP1               |
/// | 28 | ARG2 for EIP1               |
pub fn kthread_create(main: usize, arg: usize) -> Result<Arc<Task>, AllocError> {
	let mut stack = Stack::alloc()?;

	stack.push(arg).unwrap();
	stack.push(main).unwrap();
	stack.push(0).unwrap();
	stack.push(kthread_entry as usize).unwrap();
	stack.push(0).unwrap();
	stack.push(0).unwrap();
	stack.push(0).unwrap();
	stack.push(0).unwrap();

	let task = Task::alloc_new(stack)?;

	Ok(task)
}

pub fn sys_exit(_status: usize) {
	let current = unsafe { CURRENT.get_mut() };

	*current.state.lock() = State::Exited;
	yield_now();

	// unreachable!();
}

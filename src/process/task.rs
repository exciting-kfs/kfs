use core::alloc::AllocError;
use core::mem;
use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::{collections::LinkedList, sync::Arc};

use crate::config::{USER_CODE_BASE, USTACK_BASE, USTACK_PAGES};
use crate::interrupt::InterruptFrame;
use crate::mm::user::memory::Memory;
use crate::sync::locked::{Locked, LockedGuard};
use crate::sync::{cpu_local::CpuLocal, singleton::Singleton};

use super::kstack::Stack;

pub static CURRENT: CpuLocal<Arc<Task>> = CpuLocal::uninit();
pub static TASK_QUEUE: Singleton<LinkedList<Arc<Task>>> = Singleton::new(LinkedList::new());

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
	Running,
	Sleeping,
	Exited,
}

#[repr(C)]
pub struct Task {
	kstack: Stack,
	state: Locked<State>,
	memory: Option<Locked<Memory>>,
	pid: usize,
}

static LAST_PID: AtomicUsize = AtomicUsize::new(1);

impl Task {
	pub fn new_user(code: &[u8]) -> Result<Arc<Self>, AllocError> {
		let pid = LAST_PID.fetch_add(1, Ordering::Relaxed);

		let kstack = Stack::new_user(USER_CODE_BASE, USTACK_BASE)?;
		let memory = Memory::new(USTACK_BASE, USTACK_PAGES, USER_CODE_BASE, code)?;

		Ok(Arc::new(Task {
			kstack,
			state: Locked::new(State::Running),
			memory: Some(Locked::new(memory)),
			pid,
		}))
	}

	pub fn new_kernel(routine: usize, arg: usize) -> Result<Arc<Self>, AllocError> {
		let kstack = Stack::new_kernel(routine, arg)?;

		Ok(Arc::new(Task {
			kstack,
			state: Locked::new(State::Running),
			memory: None,
			pid: 0,
		}))
	}

	pub fn new_kernel_from_raw(kstack: Stack) -> Arc<Self> {
		Arc::new(Task {
			kstack,
			state: Locked::new(State::Running),
			memory: None,
			pid: 0,
		})
	}

	pub fn clone_for_fork(&self, frame: *mut InterruptFrame) -> Result<Arc<Self>, AllocError> {
		let pid = LAST_PID.fetch_add(1, Ordering::Relaxed);

		unsafe { (*frame).eax = mem::transmute(-1) };

		let kstack = self.kstack.clone_for_fork(frame)?;
		let memory = self.memory.as_ref().unwrap().lock().clone()?;

		unsafe { (*frame).eax = pid };

		Ok(Arc::new(Task {
			kstack,
			state: Locked::new(State::Running),
			memory: Some(Locked::new(memory)),
			pid,
		}))
	}

	pub fn get_pid(&self) -> usize {
		self.pid
	}

	pub fn get_uid(&self) -> usize {
		0
	}

	pub fn lock_state(&self) -> LockedGuard<'_, State> {
		self.state.lock()
	}

	pub fn lock_memory(&self) -> Option<LockedGuard<'_, Memory>> {
		self.memory.as_ref().map(|x| x.lock())
	}

	pub fn kstack_base(&self) -> usize {
		self.kstack.base()
	}
}

extern "C" {
	pub fn return_from_fork();
}

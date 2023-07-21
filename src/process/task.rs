use core::alloc::AllocError;
use core::array;

use alloc::{collections::LinkedList, sync::Arc};

use crate::config::{USER_CODE_BASE, USTACK_BASE, USTACK_PAGES};
use crate::file::File;
use crate::interrupt::InterruptFrame;
use crate::mm::user::memory::Memory;
use crate::process::context::{context_switch, InContext};
use crate::sync::locked::{Locked, LockedGuard};
use crate::sync::{cpu_local::CpuLocal, singleton::Singleton};

use super::family::TASK_TREE;
use super::kstack::Stack;
use super::pid::Pid;

pub static CURRENT: CpuLocal<Arc<Task>> = CpuLocal::uninit();
pub static TASK_QUEUE: Singleton<LinkedList<Arc<Task>>> = Singleton::new(LinkedList::new());

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
	Running,
	Sleeping,
	Exited,
}

const FDTABLE_SIZE: usize = 256;

#[repr(C)]
pub struct Task {
	// caution: kstack must always be offset = 0x0
	kstack: Stack,
	state: Locked<State>,
	pub fd_table: Locked<[Option<Arc<File>>; FDTABLE_SIZE]>,
	user_ext: Option<UserTaskExt>,
	pid: Pid,
}

unsafe impl Sync for Task {}
unsafe impl Send for Task {}

pub struct UserTaskExt {
	memory: Locked<Memory>,
}

unsafe impl Sync for UserTaskExt {}
unsafe impl Send for UserTaskExt {}

impl UserTaskExt {
	pub fn lock_memory(&self) -> LockedGuard<'_, Memory> {
		self.memory.lock()
	}
}

impl Task {
	pub fn new_user(code: &[u8]) -> Result<Arc<Self>, AllocError> {
		let kstack = Stack::new_user(USER_CODE_BASE, USTACK_BASE)?;
		let memory = Memory::new(USTACK_BASE, USTACK_PAGES, USER_CODE_BASE, code)?;

		let pid = TASK_TREE.lock().add_init_task();

		Ok(Arc::new(Task {
			pid,
			kstack,
			state: Locked::new(State::Running),
			fd_table: Locked::new(array::from_fn(|_| None)),
			user_ext: Some(UserTaskExt {
				memory: Locked::new(memory),
			}),
		}))
	}

	pub fn new_kernel(routine: usize, arg: usize) -> Result<Arc<Self>, AllocError> {
		let pid = Pid::allocate();
		let kstack = Stack::new_kernel(routine, arg)?;

		Ok(Self::new_kernel_from_raw(pid, kstack))
	}

	pub fn new_kernel_from_raw(pid: Pid, kstack: Stack) -> Arc<Self> {
		Arc::new(Task {
			pid,
			kstack,
			state: Locked::new(State::Running),
			fd_table: Locked::new(array::from_fn(|_| None)),
			user_ext: None,
		})
	}

	pub fn get_user_ext(&self) -> Option<&UserTaskExt> {
		self.user_ext.as_ref()
	}

	pub fn clone_for_fork(
		self: &Arc<Self>,
		frame: *const InterruptFrame,
	) -> Result<Arc<Self>, AllocError> {
		let kstack = self.kstack.clone_for_fork(frame)?;
		let user_ext = self.get_user_ext().unwrap();

		let memory = user_ext.memory.lock().clone()?;

		let pid = TASK_TREE.lock().add_child_task(self.get_pid());

		Ok(Arc::new(Task {
			pid,
			kstack,
			state: Locked::new(State::Running),
			fd_table: Locked::new(array::from_fn(|_| None)),
			user_ext: Some(UserTaskExt {
				memory: Locked::new(memory),
			}),
		}))
	}

	pub fn get_pid(&self) -> Pid {
		self.pid
	}

	pub fn get_uid(&self) -> usize {
		0
	}

	pub fn lock_state(&self) -> LockedGuard<'_, State> {
		self.state.lock()
	}

	pub fn kstack_base(&self) -> usize {
		self.kstack.base()
	}

	pub fn is_kernel(&self) -> bool {
		self.user_ext.is_none()
	}
}

extern "C" {
	pub fn return_from_interrupt();
}

pub extern "C" fn return_from_fork() {
	context_switch(InContext::User);
}

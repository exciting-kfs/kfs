use core::alloc::AllocError;
use core::sync::atomic::{AtomicBool, Ordering};

use alloc::collections::BTreeMap;
use alloc::{collections::LinkedList, sync::Arc};

use crate::config::{USER_CODE_BASE, USTACK_BASE, USTACK_PAGES};
use crate::interrupt::syscall::errno::Errno;
use crate::interrupt::InterruptFrame;
use crate::mm::user::memory::Memory;
use crate::process::context::{context_switch, InContext};
use crate::process::relation::family::zombie::Zombie;
use crate::signal::Signal;
use crate::sync::locked::{Locked, LockedGuard};
use crate::sync::{cpu_local::CpuLocal, singleton::Singleton};

use super::exit::ExitStatus;
use super::fd_table::FdTable;
use super::kstack::Stack;
use super::relation::{Pgid, Pid, Relation, Sid};
use super::wait::Who;

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
	// caution: kstack must always be offset = 0x0
	kstack: Stack,
	state: Locked<State>,
	pid: Pid,
	user_ext: Option<UserTaskExt>,
}

unsafe impl Sync for Task {}
unsafe impl Send for Task {}

pub struct UserTaskExt {
	exec_called: AtomicBool,
	memory: Locked<Memory>,
	relation: Locked<Relation>,
	fd_table: Arc<Locked<FdTable>>,
	pub signal: Arc<Signal>,
}

unsafe impl Sync for UserTaskExt {}
unsafe impl Send for UserTaskExt {}

impl UserTaskExt {
	pub fn lock_memory(&self) -> LockedGuard<'_, Memory> {
		self.memory.lock()
	}

	pub fn lock_relation(&self) -> LockedGuard<'_, Relation> {
		self.relation.lock()
	}

	pub fn lock_fd_table(&self) -> LockedGuard<'_, FdTable> {
		self.fd_table.lock()
	}

	pub fn was_exec_called(&self) -> bool {
		self.exec_called.load(Ordering::SeqCst)
	}

	pub fn set_exec_called(&self) {
		self.exec_called.store(true, Ordering::SeqCst);
	}
}

impl Task {
	/// create new init (pid 1) process.
	/// this must be called only once!!
	pub(super) fn new_init_task(code: &[u8]) -> Result<Arc<Self>, AllocError> {
		let kstack = Stack::new_user(USER_CODE_BASE, USTACK_BASE)?;
		let memory = Memory::new(USTACK_BASE, USTACK_PAGES, USER_CODE_BASE, code)?;

		let task = Arc::new(Task {
			kstack,
			state: Locked::new(State::Running),
			pid: Pid::from_raw(1),
			user_ext: Some(UserTaskExt {
				exec_called: AtomicBool::new(false),
				memory: Locked::new(memory),
				relation: Locked::new(Relation::new_init()),
				fd_table: Arc::new(Locked::new(FdTable::new())),
				signal: Arc::new(Signal::new()),
			}),
		});

		let mut ptree = PROCESS_TREE.lock();
		ptree.insert(task.clone());

		Ok(task)
	}

	pub fn new_kernel(routine: usize, arg: usize) -> Result<Arc<Self>, AllocError> {
		let pid = Pid::allocate();
		let kstack = Stack::new_kernel(routine, arg)?;

		Ok(Self::new_kernel_from_raw(pid, kstack))
	}

	pub(super) fn new_kernel_from_raw(pid: Pid, kstack: Stack) -> Arc<Self> {
		let task = Arc::new(Task {
			kstack,
			state: Locked::new(State::Running),
			pid,
			user_ext: None,
		});

		let mut ptree = PROCESS_TREE.lock();
		ptree.insert(task.clone());

		task
	}

	pub fn get_user_ext(&self) -> Option<&UserTaskExt> {
		self.user_ext.as_ref()
	}

	pub fn clone_for_fork(
		self: &Arc<Self>,
		frame: *const InterruptFrame,
	) -> Result<Arc<Self>, AllocError> {
		let kstack = self.kstack.clone_for_fork(frame)?;
		let pid = Pid::allocate();

		let user_ext = self.get_user_ext().unwrap();

		let memory = user_ext.lock_memory().clone()?;
		let relation = user_ext.lock_relation().clone_for_fork(pid, self.pid);
		let fd_table = user_ext.lock_fd_table().clone_for_fork();
		let signal = user_ext.signal.clone_for_fork();

		let new_task = Arc::new(Task {
			kstack,
			state: Locked::new(State::Running),
			pid,
			user_ext: Some(UserTaskExt {
				exec_called: AtomicBool::new(false),
				memory: Locked::new(memory),
				relation: Locked::new(relation),
				fd_table: Arc::new(Locked::new(fd_table)),
				signal: Arc::new(signal),
			}),
		});

		let mut ptree = PROCESS_TREE.lock();
		ptree.insert(new_task.clone());

		Ok(new_task)
	}

	pub fn get_pid(&self) -> Pid {
		self.pid
	}

	pub fn get_ppid(&self) -> Pid {
		self.get_user_ext()
			.map(|ext| ext.lock_relation().get_ppid())
			.unwrap_or_else(|| Pid::from_raw(0))
	}

	pub fn get_uid(&self) -> usize {
		0 // TODO
	}

	pub fn get_pgid(&self) -> Pgid {
		self.get_user_ext()
			.map(|ext| ext.lock_relation().get_pgid())
			.unwrap_or_else(|| Pgid::from_raw(0))
	}

	pub fn set_pgid(&self, pgid: Pgid) -> Result<(), Errno> {
		let ext = self.get_user_ext().ok_or_else(|| Errno::EINVAL)?;

		// already called `exec(2)`.
		if ext.was_exec_called() {
			return Err(Errno::EACCES);
		}

		let mut relation = ext.lock_relation();

		relation.set_pgid(self.pid, pgid)
	}

	pub fn get_sid(&self) -> Sid {
		self.get_user_ext()
			.map(|ext| ext.lock_relation().get_sid())
			.unwrap_or_else(|| Sid::from_raw(0))
	}

	pub fn set_sid(&self) -> Result<usize, Errno> {
		let ext = self.get_user_ext().ok_or_else(|| Errno::EINVAL)?;

		let mut relation = ext.lock_relation();

		relation.set_sid(self.pid)
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

	pub fn exit(&self, status: ExitStatus) {
		*self.lock_state() = State::Exited;

		PROCESS_TREE.lock().remove(&self.pid);

		if self.is_kernel() {
			Pid::deallocate(self.get_pid());
			return;
		}

		let ext = self.get_user_ext().unwrap();
		ext.lock_relation().exit(self.pid, status);
	}

	pub fn waitpid(&self, who: Who) -> Result<Zombie, Errno> {
		let mut relation = self
			.get_user_ext()
			.expect("kernel thread has no relation.")
			.lock_relation();

		let result = relation.waitpid(who);
		if let Ok(z) = result {
			Pid::deallocate(z.pid);
		}

		result
	}
}

extern "C" {
	pub fn return_from_interrupt();
}

pub extern "C" fn return_from_fork() {
	context_switch(InContext::User);
}

pub struct ProcessTree(BTreeMap<Pid, Arc<Task>>);
pub static PROCESS_TREE: Singleton<ProcessTree> = Singleton::new(ProcessTree::new());

impl ProcessTree {
	pub const fn new() -> Self {
		Self(BTreeMap::new())
	}

	pub fn insert(&mut self, task: Arc<Task>) {
		self.0.insert(task.get_pid(), task);
	}

	pub fn remove(&mut self, pid: &Pid) {
		self.0.remove(pid);
	}

	pub fn get(&self, pid: &Pid) -> Option<&Arc<Task>> {
		self.0.get(pid)
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

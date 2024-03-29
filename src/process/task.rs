use core::alloc::AllocError;
use core::sync::atomic::{AtomicBool, Ordering};

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::config::USTACK_BASE;
use crate::elf::Elf;
use crate::fs::vfs::{VfsDirEntry, ROOT_DIR_ENTRY};
use crate::fs::{create_task_node, delete_task_node};
use crate::interrupt::InterruptFrame;
use crate::mm::user::memory::Memory;
use crate::mm::user::string_vec::StringVec;
use crate::process::relation::family::zombie::Zombie;
use crate::process::signal::sig_info::SigInfo;
use crate::process::signal::sig_num::SigNum;
use crate::process::signal::Signal;
use crate::scheduler::sleep::{wake_up, Sleep};
use crate::sync::CpuLocal;
use crate::sync::{Locked, LockedGuard};
use crate::syscall::errno::Errno;
use crate::syscall::wait::Who;
use crate::x86::SystemDesc;

use super::exit::ExitStatus;
use super::fd_table::FdTable;
use super::gid::Gid;
use super::kstack::Stack;
use super::process_tree::PROCESS_TREE;
use super::relation::{Pgid, Pid, Relation, Sid};
use super::uid::Uid;

pub static CURRENT: CpuLocal<Arc<Task>> = CpuLocal::uninit();

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
	Running,
	Sleeping,
	DeepSleep,
	Exited,
}

#[repr(C)]
pub struct Task {
	// caution: kstack must always be offset = 0x0
	kstack: Stack,
	state: Locked<State>,
	cmd: Locked<Vec<u8>>,
	pid: Pid,
	uid: Uid,
	gid: Gid,
	user_ext: Option<UserTaskExt>,
}

unsafe impl Sync for Task {}
unsafe impl Send for Task {}

pub struct UserTaskExt {
	exec_called: AtomicBool,
	cwd: Locked<Arc<VfsDirEntry>>,
	memory: Locked<Memory>,
	relation: Locked<Relation>,
	fd_table: Arc<Locked<FdTable>>,
	pub signal: Arc<Signal>,
	tls: Locked<[SystemDesc; 3]>,
}

unsafe impl Sync for UserTaskExt {}
unsafe impl Send for UserTaskExt {}

impl UserTaskExt {
	pub fn lock_memory(&self) -> LockedGuard<'_, Memory> {
		self.memory.lock()
	}

	pub fn lock_cwd(&self) -> LockedGuard<'_, Arc<VfsDirEntry>> {
		self.cwd.lock()
	}

	pub fn lock_relation(&self) -> LockedGuard<'_, Relation> {
		self.relation.lock()
	}

	pub fn lock_fd_table(&self) -> LockedGuard<'_, FdTable> {
		self.fd_table.lock()
	}

	pub fn lock_tls(&self) -> LockedGuard<'_, [SystemDesc; 3]> {
		self.tls.lock()
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
	pub(super) fn new_init_task(pid: Pid, elf: Elf<'_>) -> Result<Arc<Self>, Errno> {
		debug_assert!(pid.as_raw() == 1, "invalid init pid");

		let kstack =
			Stack::new_user(elf.get_entry_point(), USTACK_BASE - 32).map_err(|_| Errno::ENOMEM)?;
		let memory = Memory::from_elf(elf, StringVec::new_null(), StringVec::new_null())?;

		let task = Arc::new_cyclic(|w| Task {
			kstack,
			state: Locked::new(State::Running),
			pid,
			uid: Uid::from_raw(0),
			gid: Gid::from_raw(0),
			cmd: Locked::new(b"init".to_vec()),
			user_ext: Some(UserTaskExt {
				exec_called: AtomicBool::new(false),
				cwd: Locked::new(ROOT_DIR_ENTRY.lock().as_ref().unwrap().clone()),
				memory: Locked::new(memory),
				relation: Locked::new(Relation::new_init(w)),
				fd_table: Arc::new(Locked::new(FdTable::new())),
				signal: Arc::new(Signal::new()),
				tls: Locked::new([SystemDesc::new_null(); 3]),
			}),
		});

		let mut ptree = PROCESS_TREE.lock();
		ptree.insert(task.clone());

		Ok(task)
	}

	pub fn new_kernel(routine: usize, arg: usize) -> Result<Arc<Self>, AllocError> {
		let pid = Pid::allocate();
		let kstack = Stack::new_kernel(routine, arg)?;

		let task = Self::new_kernel_from_raw(pid, kstack);

		let _ = create_task_node(&task);

		Ok(task)
	}

	pub(super) fn new_kernel_from_raw(pid: Pid, kstack: Stack) -> Arc<Self> {
		let task = Arc::new(Task {
			kstack,
			state: Locked::new(State::Running),
			pid,
			uid: Uid::from_raw(0),
			gid: Gid::from_raw(0),
			cmd: Locked::new(b"kthread".to_vec()),
			user_ext: None,
		});

		let mut ptree = PROCESS_TREE.lock();
		ptree.insert(task.clone());

		task
	}

	#[inline]
	pub fn get_user_ext(&self) -> Option<&UserTaskExt> {
		self.user_ext.as_ref()
	}

	#[inline]
	pub fn user_ext_ok_or<E>(&self, e: E) -> Result<&UserTaskExt, E> {
		self.user_ext.as_ref().ok_or(e)
	}

	pub fn clone_for_fork(
		self: &Arc<Self>,
		frame: *const InterruptFrame,
	) -> Result<Arc<Self>, AllocError> {
		let kstack = self.kstack.clone_for_fork(frame)?;
		let pid = Pid::allocate();
		let uid = self.uid.clone();
		let gid = self.gid.clone();
		let cmd = self.cmd.lock().clone();

		let user_ext = self.get_user_ext().unwrap();

		let cwd = user_ext.lock_cwd().clone();
		let memory = user_ext.lock_memory().clone()?;
		let fd_table = user_ext.lock_fd_table().clone_for_fork();
		let signal = user_ext.signal.clone_for_fork();
		let tls = user_ext.tls.lock().clone();

		let new_task = Arc::new_cyclic(|w| {
			let relation = user_ext
				.lock_relation()
				.clone_for_fork(pid, self.pid, w.clone());

			Task {
				kstack,
				state: Locked::new(State::Running),
				pid,
				uid,
				gid,
				cmd: Locked::new(cmd),
				user_ext: Some(UserTaskExt {
					exec_called: AtomicBool::new(false),
					cwd: Locked::new(cwd),
					memory: Locked::new(memory),
					relation: Locked::new(relation),
					fd_table: Arc::new(Locked::new(fd_table)),
					signal: Arc::new(signal),
					tls: Locked::new(tls),
				}),
			}
		});

		let mut ptree = PROCESS_TREE.lock();
		ptree.insert(new_task.clone());

		let _ = create_task_node(&new_task);

		Ok(new_task)
	}

	pub fn lock_cmd(&self) -> LockedGuard<'_, Vec<u8>> {
		self.cmd.lock()
	}

	#[inline]
	pub fn get_pid(&self) -> Pid {
		self.pid
	}

	pub fn get_ppid(&self) -> Pid {
		self.get_user_ext()
			.map(|ext| ext.lock_relation().get_ppid())
			.unwrap_or_else(|| Pid::from_raw(0))
	}

	pub fn get_uid(&self) -> usize {
		self.uid.as_raw()
	}

	pub fn get_gid(&self) -> usize {
		self.gid.as_raw()
	}

	pub fn set_uid(&self, new_uid: usize) -> Result<(), Errno> {
		self.uid.set(new_uid)
	}

	pub fn set_gid(&self, new_gid: usize) -> Result<(), Errno> {
		if self.get_gid() == new_gid {
			return Ok(());
		}

		if self.get_uid() == 0 {
			self.gid.set(new_gid);
			return Ok(());
		}

		Err(Errno::EPERM)
	}

	pub fn get_pgid(&self) -> Pgid {
		self.get_user_ext()
			.map(|ext| ext.lock_relation().pgroup.get_pgid())
			.unwrap_or_default()
	}

	pub fn get_sid(&self) -> Sid {
		self.get_user_ext()
			.map(|ext| ext.lock_relation().pgroup.get_sid())
			.unwrap_or_default()
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
		let _ = delete_task_node(self.get_pid());

		let mut state = self.lock_state();
		PROCESS_TREE.lock().remove(&self.pid);

		if let Some(ref ext) = self.user_ext {
			let mut rel = ext.lock_relation();
			rel.exit(self.pid, status);
			let pgrp = &mut rel.pgroup;
			pgrp.lock_members().remove(&self.pid);
		} else {
			Pid::deallocate(self.get_pid());
		}

		*state = State::Exited;
	}

	pub fn waitpid(&self, who: Who) -> Result<Zombie, Errno> {
		let mut relation = self
			.get_user_ext()
			.expect("kernel thread has no relation.")
			.lock_relation();

		let result = relation.waitpid(who);
		if let Ok(z) = result.as_ref() {
			Pid::deallocate(z.pid);
		}

		result
	}

	pub fn recv_signal(self: &Arc<Self>, info: SigInfo) -> Result<(), Errno> {
		let signal = &self.user_ext_ok_or(Errno::EPERM)?.signal;

		let kill = info.num == SigNum::KILL;
		let cont = info.num == SigNum::CONT && signal.is_default(&SigNum::CONT);

		if kill || cont {
			wake_up(self, Sleep::Deep);
		}

		signal.recv_signal(info);
		Ok(())
	}

	pub fn is_privileged(&self) -> bool {
		self.is_kernel() || self.get_uid() == 0
	}
}

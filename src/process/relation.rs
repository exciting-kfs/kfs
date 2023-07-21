pub mod family;
mod id;
pub mod job;

use core::mem;

pub use id::*;
use kfs_macro::context;

use crate::{interrupt::syscall::errno::Errno, process::task::CURRENT};

use self::family::{zombie::Zombie, Family};
use self::job::JobGroup;

use super::exit::ExitStatus;
use super::task::PROCESS_TREE;
use super::wait::Who;

pub struct Relation {
	family: Family,
	jobgroup: JobGroup,
}

impl Relation {
	pub fn new_init() -> Self {
		Self {
			family: Family::new(Pid::from_raw(0)),
			jobgroup: JobGroup::new_init(),
		}
	}

	pub fn clone_for_fork(&mut self, pid: Pid, ppid: Pid) -> Self {
		self.family.insert_child(pid);

		Self {
			family: Family::new(ppid),
			jobgroup: self.jobgroup.clone_for_fork(pid),
		}
	}

	pub fn get_ppid(&self) -> Pid {
		self.family.get_ppid()
	}

	pub fn get_pgid(&self) -> Pgid {
		self.jobgroup.get_pgid()
	}

	pub fn set_pgid(&mut self, pid: Pid, pgid: Pgid) -> Result<(), Errno> {
		self.jobgroup.set_pgid(pid, pgid)
	}

	pub fn get_sid(&self) -> Sid {
		self.jobgroup.get_sid()
	}

	pub fn set_sid(&mut self, pid: Pid) -> Result<usize, Errno> {
		self.jobgroup.set_sid(pid)
	}

	pub fn waitpid(&mut self, who: Who) -> Result<Zombie, Errno> {
		let result = match who {
			Who::Any => self.family.wait_any(),
			Who::Pid(x) => self.family.wait_pid(x),
			Who::Pgid(x) => self.family.wait_pgid(x),
		};

		result
	}

	pub fn exit(&mut self, pid: Pid, status: ExitStatus) {
		let zombie = Zombie::new(pid, self.get_pgid(), status);

		self.family.exit(zombie);
		self.jobgroup.exit(pid);
	}

	pub fn update_child_to_zombie(&mut self, zombie: Zombie) {
		self.family.update_child_to_zombie(zombie);
	}

	pub fn update_parent_to_init(&mut self) {
		self.family.update_parent_to_init();
	}
}

#[context(irq_disabled)]
pub fn sys_getpid() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	Ok(current.get_pid().as_raw())
}

#[context(irq_disabled)]
pub fn sys_setpgid(pid: usize, pgid: usize) -> Result<usize, Errno> {
	let task = if pid == 0 {
		unsafe { CURRENT.get_mut().clone() }
	} else {
		let ptree = PROCESS_TREE.lock();

		ptree
			.get(&Pid::from_raw(pid))
			.ok_or_else(|| Errno::ESRCH)?
			.clone()
	};

	let pgid = if pgid == 0 {
		task.get_pid().as_raw()
	} else {
		pgid
	};

	task.set_pgid(Pgid::from_raw(pgid)).map(|_| 0)
}

#[context(irq_disabled)]
pub fn sys_getppid() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	Ok(current.get_ppid().as_raw())
}

#[context(irq_disabled)]
pub fn sys_getpgrp() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	Ok(current.get_pgid().as_raw())
}

#[context(irq_disabled)]
pub fn sys_setsid() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	current.set_sid()
}

#[context(irq_disabled)]
pub fn sys_getpgid(pid: usize) -> Result<usize, Errno> {
	let ptree = PROCESS_TREE.lock();

	let task = ptree
		.get(&Pid::from_raw(pid))
		.ok_or_else(|| Errno::ESRCH)?
		.clone();

	mem::drop(ptree);

	Ok(task.get_pgid().as_raw())
}

#[context(irq_disabled)]
pub fn sys_getsid() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	Ok(current.get_sid().as_raw())
}

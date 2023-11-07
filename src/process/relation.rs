pub mod family;
pub mod pgroup;
pub mod session;
pub mod syscall;

mod id;

pub use id::*;

use alloc::sync::{Arc, Weak};

use crate::pr_debug;
use crate::sync::Locked;
use crate::syscall::errno::Errno;
use crate::syscall::wait::Who;

use self::family::{zombie::Zombie, Family};
use self::pgroup::ProcessGroup;
use self::session::Session;

use super::exit::ExitStatus;
use super::task::Task;

pub struct Relation {
	family: Family,
	pub pgroup: Arc<ProcessGroup>,
}

impl Relation {
	pub fn new_init(w: &Weak<Task>) -> Self {
		Self {
			family: Family::new(Pid::from_raw(0)),
			pgroup: ProcessGroup::new_init(w),
		}
	}

	pub fn clone_for_fork(&mut self, pid: Pid, ppid: Pid, weak: Weak<Task>) -> Self {
		self.family.insert_child(pid);

		Self {
			family: Family::new(ppid),
			pgroup: self.pgroup.clone_for_fork(pid, weak),
		}
	}

	pub fn get_ppid(&self) -> Pid {
		self.family.get_ppid()
	}

	pub fn get_pgroup(&self) -> Arc<ProcessGroup> {
		self.pgroup.clone()
	}

	pub fn get_session(&self) -> Arc<Locked<Session>> {
		self.pgroup.sess.clone()
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
		let zombie = Zombie::new(pid, self.get_pgroup(), self.get_session(), status);
		self.family.exit(zombie);
	}

	pub fn update_child_to_zombie(&mut self, zombie: Zombie) {
		self.family.update_child_to_zombie(zombie);
	}

	pub fn update_parent_to_init(&mut self) {
		self.family.update_parent_to_init();
	}

	pub fn enter_new_pgroup(&mut self, pid: Pid, new: Arc<ProcessGroup>) {
		let weak = self
			.pgroup
			.lock_members()
			.remove(&pid)
			.expect("task in pgroup.");
		new.lock_members().insert(pid, weak);

		pr_debug!("MOVE: {:?} is now in {:?}", pid, new.get_pgid());
		self.pgroup = new;
	}

	pub fn enter_new_session(&mut self, pid: Pid) {
		let pgid = Pgid::from(pid);

		let sess = Arc::new(Locked::new(Session::new(Sid::from(pid))));
		let pgrp = Session::new_pgroup(&sess, pgid);
		self.enter_new_pgroup(pid, pgrp)
	}
}

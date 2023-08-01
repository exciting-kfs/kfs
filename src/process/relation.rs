pub mod family;
pub mod job;
pub mod syscall;

mod id;

pub use id::*;

use alloc::sync::{Arc, Weak};

use crate::interrupt::syscall::errno::Errno;
use crate::sync::locked::Locked;

use self::family::{zombie::Zombie, Family};
use self::job::group::ProcessGroup;
use self::job::session::Session;
use self::job::JobGroup;

use super::exit::ExitStatus;
use super::task::Task;
use super::wait::Who;

pub struct Relation {
	family: Family,
	pub jobgroup: JobGroup,
}

impl Relation {
	pub fn new_init(w: &Weak<Task>) -> Self {
		Self {
			family: Family::new(Pid::from_raw(0)),
			jobgroup: JobGroup::new_init(w),
		}
	}

	pub fn clone_for_fork(&mut self, pid: Pid, ppid: Pid, weak: Weak<Task>) -> Self {
		self.family.insert_child(pid);

		Self {
			family: Family::new(ppid),
			jobgroup: self.jobgroup.clone_for_fork(pid, weak),
		}
	}

	pub fn get_ppid(&self) -> Pid {
		self.family.get_ppid()
	}

	pub fn get_pgroup(&self) -> Arc<ProcessGroup> {
		self.jobgroup.pgroup.clone()
	}

	pub fn get_session(&self) -> Arc<Locked<Session>> {
		self.jobgroup.pgroup.sess.clone()
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
		let zombie = Zombie::new(pid, self.jobgroup.pgroup.get_pgid(), status);
		self.family.exit(zombie);
	}

	pub fn update_child_to_zombie(&mut self, zombie: Zombie) {
		self.family.update_child_to_zombie(zombie);
	}

	pub fn update_parent_to_init(&mut self) {
		self.family.update_parent_to_init();
	}
}

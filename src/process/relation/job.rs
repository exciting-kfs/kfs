pub mod group;
pub mod session;

use core::mem;

use alloc::sync::Arc;

use crate::{interrupt::syscall::errno::Errno, sync::locked::Locked};

use self::{
	group::ProcessGroup,
	session::{Session, SessionTree},
};

use super::{Pgid, Pid, Sid};

pub static SESSION_TREE: Locked<SessionTree> = Locked::new(SessionTree::new());

#[derive(Clone)]
pub struct JobGroup {
	session: Arc<Locked<Session>>,
	pgroup: Arc<Locked<ProcessGroup>>,
}

impl JobGroup {
	pub fn exit(&self, pid: Pid) {
		let mut pgrp_lock = self.pgroup.lock();

		pgrp_lock.remove(&pid);
		if pgrp_lock.is_empty() {
			self.exit_session(pgrp_lock.get_pgid());
		}
	}

	fn exit_session(&self, pgid: Pgid) {
		let mut sess_lock = self.session.lock();

		sess_lock.remove(&pgid);
		if sess_lock.is_empty() {
			self.exit_session_tree(sess_lock.get_sid());
		}
	}

	fn exit_session_tree(&self, sid: Sid) {
		let mut sess_tree_lock = SESSION_TREE.lock();

		sess_tree_lock.remove(&sid);
	}

	pub fn set_sid(&mut self, pid: Pid) -> Result<usize, Errno> {
		if self.get_sid() == Sid::from_raw(pid.as_raw()) {
			return Err(Errno::EPERM);
		}

		self.change_session(pid);
		Ok(pid.as_raw())
	}

	pub fn set_pgid(&mut self, pid: Pid, new_pgid: Pgid) -> Result<(), Errno> {
		if self.get_pgid() == new_pgid {
			return Ok(());
		}

		let sess_lock = self.session.lock();
		if sess_lock.get(&new_pgid).is_some() || pid.as_raw() == new_pgid.as_raw() {
			mem::drop(sess_lock);
			self.change_pgroup(pid, new_pgid);
			return Ok(());
		}

		Err(Errno::EPERM)
	}

	fn change_pgroup(&mut self, pid: Pid, pgid: Pgid) {
		self.exit(pid);

		let mut sess_lock = self.session.lock();
		let pgrp = sess_lock.get_or_insert(pgid);

		let mut pgrp_lock = pgrp.lock();
		pgrp_lock.insert(pid);

		self.pgroup = pgrp.clone();
	}

	fn change_session(&mut self, pid: Pid) {
		self.exit(pid);

		let mut sess_tree_lock = SESSION_TREE.lock();
		let sess = sess_tree_lock.get_or_insert(Sid::from_raw(pid.as_raw()));

		let mut sess_lock = sess.lock();
		let pgrp = sess_lock.get_or_insert(Pgid::from_raw(pid.as_raw()));

		let mut pgrp_lock = pgrp.lock();
		pgrp_lock.insert(pid);

		self.session = sess.clone();
		self.pgroup = pgrp.clone();
	}

	pub fn new_init() -> Self {
		let mut sess_tree_lock = SESSION_TREE.lock();
		let init_sess = sess_tree_lock.get_or_insert(Sid::from_raw(1));

		let mut init_sess_lock = init_sess.lock();
		let init_pgrp = init_sess_lock.get_or_insert(Pgid::from_raw(1));

		let mut init_pgrp_lock = init_pgrp.lock();
		init_pgrp_lock.insert(Pid::from_raw(1));

		Self {
			session: init_sess.clone(),
			pgroup: init_pgrp.clone(),
		}
	}

	pub fn clone_for_fork(&self, pid: Pid) -> Self {
		let pgroup = self.pgroup.clone();

		let mut pgroup_lock = pgroup.lock();
		pgroup_lock.insert(pid);
		mem::drop(pgroup_lock);

		Self {
			session: self.session.clone(),
			pgroup,
		}
	}

	pub fn get_pgid(&self) -> Pgid {
		let pgroup = self.pgroup.lock();
		pgroup.get_pgid()
	}

	pub fn get_sid(&self) -> Sid {
		let session = self.session.lock();
		session.get_sid()
	}
}

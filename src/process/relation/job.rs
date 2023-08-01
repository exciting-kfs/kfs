pub mod group;
pub mod session;

use alloc::sync::{Arc, Weak};

use crate::{process::task::Task, sync::locked::Locked};

use self::{group::ProcessGroup, session::Session};

use super::{Pgid, Pid, Sid};

#[derive(Clone)]
pub struct JobGroup {
	pub pgroup: Arc<ProcessGroup>,
}

impl JobGroup {
	pub fn enter_new_pgroup(&mut self, pid: Pid, new: Arc<ProcessGroup>) {
		let weak = self
			.pgroup
			.lock_members()
			.remove(&pid)
			.expect("task in pgroup.");
		new.lock_members().insert(pid, weak);
		self.pgroup = new;
	}

	pub fn new_pgroup_in_session(pgid: Pgid, sess: &Arc<Locked<Session>>) -> Arc<ProcessGroup> {
		let pgrp = Arc::new(ProcessGroup::new(pgid, sess.clone()));
		let weak = Arc::downgrade(&pgrp);

		sess.lock().insert(pgid, weak);
		pgrp
	}

	pub fn enter_new_session(&mut self, pid: Pid) {
		let pgid = Pgid::from(pid);

		let sess = Arc::new(Locked::new(Session::new(Sid::from(pid))));
		let pgrp = Self::new_pgroup_in_session(pgid, &sess);
		self.enter_new_pgroup(pid, pgrp)
	}

	pub fn new_init(w: &Weak<Task>) -> Self {
		let pid = Pid::from_raw(1);
		let sid = Sid::from(pid);
		let pgid = Pgid::from(pid);

		let sess = Arc::new(Locked::new(Session::new(sid)));
		let pgrp = Arc::new(ProcessGroup::new(pgid, sess.clone()));
		let weak = Arc::downgrade(&pgrp);

		sess.lock().insert(pgid, weak);
		pgrp.lock_members().insert(pid, w.clone());

		Self { pgroup: pgrp }
	}

	pub fn clone_for_fork(&self, pid: Pid, weak: Weak<Task>) -> Self {
		let pgroup = self.pgroup.clone();

		pgroup.lock_members().insert(pid, weak);

		Self { pgroup }
	}

	pub fn get_sid(&self) -> Sid {
		let session = self.pgroup.sess.lock();
		session.get_sid()
	}
}

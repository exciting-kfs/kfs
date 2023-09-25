use alloc::{
	collections::BTreeMap,
	sync::{Arc, Weak},
};

use crate::{
	process::{
		relation::{Pgid, Pid, Sid},
		task::Task,
	},
	sync::{Locked, LockedGuard},
};

use super::session::Session;

pub struct ProcessGroup {
	pub sess: Arc<Locked<Session>>,
	pgid: Pgid,
	members: Locked<BTreeMap<Pid, Weak<Task>>>,
}

impl ProcessGroup {
	pub fn new(pgid: Pgid, sess: Arc<Locked<Session>>) -> Self {
		use crate::pr_debug;
		pr_debug!("NEW: ProcessGroup[{}]", pgid.as_raw());
		Self {
			pgid,
			sess,
			members: Locked::new(BTreeMap::new()),
		}
	}

	pub fn new_init(w: &Weak<Task>) -> Arc<Self> {
		let pid = Pid::from_raw(1);
		let sid = Sid::from(pid);
		let pgid = Pgid::from(pid);

		let sess = Arc::new(Locked::new(Session::new(sid)));
		let pgrp = Arc::new(ProcessGroup::new(pgid, sess.clone()));
		let weak = Arc::downgrade(&pgrp);

		sess.lock().insert(pgid, weak);
		pgrp.lock_members().insert(pid, w.clone());

		pgrp
	}

	pub fn clone_for_fork(self: &Arc<Self>, pid: Pid, weak: Weak<Task>) -> Arc<Self> {
		let pgroup = self.clone();

		pgroup.lock_members().insert(pid, weak);

		pgroup
	}

	pub fn lock_members(&self) -> LockedGuard<'_, BTreeMap<Pid, Weak<Task>>> {
		self.members.lock()
	}

	pub fn get_pgid(&self) -> Pgid {
		self.pgid
	}

	pub fn get_sid(&self) -> Sid {
		let session = self.sess.lock();
		session.get_sid()
	}
}

impl Drop for ProcessGroup {
	fn drop(&mut self) {
		use crate::pr_debug;

		pr_debug!("DROP: ProcessGroup[{}]", self.pgid.as_raw());

		self.sess.lock().remove(&self.pgid);

		Pgid::deallocate(self.pgid);
	}
}

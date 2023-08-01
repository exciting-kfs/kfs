use alloc::{
	collections::BTreeMap,
	sync::{Arc, Weak},
};

use crate::{
	process::{
		relation::{Pgid, Pid},
		task::Task,
	},
	sync::locked::{Locked, LockedGuard},
};

use super::session::Session;

pub struct ProcessGroup {
	pub sess: Arc<Locked<Session>>,
	pgid: Pgid,
	members: Locked<BTreeMap<Pid, Weak<Task>>>,
}

impl ProcessGroup {
	pub fn new(pgid: Pgid, sess: Arc<Locked<Session>>) -> Self {
		Self {
			pgid,
			sess,
			members: Locked::new(BTreeMap::new()),
		}
	}

	pub fn lock_members(&self) -> LockedGuard<'_, BTreeMap<Pid, Weak<Task>>> {
		self.members.lock()
	}

	pub fn get_pgid(&self) -> Pgid {
		self.pgid
	}

	pub fn cleanup(&mut self) {
		// hmm..
		let _ = self.members.lock().extract_if(|_, v| v.upgrade().is_none());
	}
}

impl Drop for ProcessGroup {
	fn drop(&mut self) {
		use crate::pr_debug;

		pr_debug!("DROP: ProcessGroup[{}]", self.pgid.as_raw());

		self.sess.lock().remove(&self.pgid);
	}
}

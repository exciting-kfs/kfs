use alloc::sync::Weak;
use alloc::{collections::BTreeMap, sync::Arc};

use crate::process::relation::{Pgid, Pid, Sid};
use crate::sync::locked::Locked;

use super::pgroup::ProcessGroup;

pub struct Session {
	sid: Sid,
	foreground: Option<Weak<ProcessGroup>>,
	members: BTreeMap<Pgid, Weak<ProcessGroup>>,
}

impl Session {
	pub fn new(sid: Sid) -> Self {
		Self {
			sid,
			foreground: None,
			members: BTreeMap::new(),
		}
	}

	pub fn get_sid(&self) -> Sid {
		self.sid
	}

	pub fn is_leader(&self, pid: Pid) -> bool {
		self.sid.as_raw() == pid.as_raw()
	}

	pub fn find(&self, pgid: Pgid) -> Option<&Weak<ProcessGroup>> {
		self.members.get(&pgid)
	}

	pub fn insert(&mut self, pgid: Pgid, w: Weak<ProcessGroup>) {
		if let None = self.foreground {
			self.foreground = Some(w.clone());
		}
		self.members.insert(pgid, w);
	}

	pub fn foreground(&self) -> Option<Weak<ProcessGroup>> {
		self.foreground.clone()
	}

	pub fn remove(&mut self, pgid: &Pgid) {
		if let Some(pgrp) = self.foreground.as_ref().and_then(|w| w.upgrade()) {
			if pgrp.get_pgid() == *pgid {
				self.foreground = self.members.first_entry().map(|o| o.get().clone());
			}
		}
		self.members.remove(pgid);
	}

	pub fn get(&self, pgid: &Pgid) -> Option<&Weak<ProcessGroup>> {
		self.members.get(pgid)
	}

	pub fn is_empty(&self) -> bool {
		self.members.is_empty()
	}

	pub fn new_pgroup(sess: &Arc<Locked<Session>>, pgid: Pgid) -> Arc<ProcessGroup> {
		let pgrp = Arc::new(ProcessGroup::new(pgid, sess.clone()));
		let weak = Arc::downgrade(&pgrp);

		sess.lock().insert(pgid, weak);
		pgrp
	}
}

use alloc::{collections::BTreeMap, sync::Arc};

use crate::process::relation::{Pgid, Pid, Sid};
use crate::sync::locked::Locked;

use super::group::{ProcessGroup, ProcessGroupTree};

type Shared<T> = Arc<Locked<T>>;

pub struct Session {
	sid: Sid,
	members: ProcessGroupTree,
}

impl Session {
	pub fn new(sid: Sid) -> Self {
		Self {
			sid,
			members: ProcessGroupTree::new(),
		}
	}

	pub fn get_sid(&self) -> Sid {
		self.sid
	}

	pub fn is_leader(&self, pid: Pid) -> bool {
		self.sid.as_raw() == pid.as_raw()
	}

	pub fn get_or_insert(&mut self, pgid: Pgid) -> &Shared<ProcessGroup> {
		self.members.get_or_insert(pgid)
	}

	pub fn remove(&mut self, pgid: &Pgid) {
		self.members.remove(pgid);
	}

	pub fn get(&self, pgid: &Pgid) -> Option<&Shared<ProcessGroup>> {
		self.members.get(pgid)
	}

	pub fn is_empty(&self) -> bool {
		self.members.is_empty()
	}
}

pub struct SessionTree(BTreeMap<Sid, Shared<Session>>);

impl SessionTree {
	pub const fn new() -> Self {
		Self(BTreeMap::new())
	}

	pub fn get_or_insert(&mut self, sid: Sid) -> &Shared<Session> {
		self.0
			.entry(sid)
			.or_insert_with(|| Arc::new(Locked::new(Session::new(sid))))
	}

	pub fn remove(&mut self, sid: &Sid) {
		self.0.remove(sid);
	}

	pub fn get(&self, sid: &Sid) -> Option<&Shared<Session>> {
		self.0.get(sid)
	}
}

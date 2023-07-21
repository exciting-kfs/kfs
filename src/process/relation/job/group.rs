use alloc::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};

use crate::{
	process::{
		relation::{Pgid, Pid},
	},
	sync::locked::Locked,
};

pub struct ProcessGroup {
	pgid: Pgid,
	members: BTreeSet<Pid>,
}

impl ProcessGroup {
	pub fn new(pgid: Pgid) -> Self {
		Self {
			pgid,
			members: BTreeSet::new(),
		}
	}

	pub fn get_pgid(&self) -> Pgid {
		self.pgid
	}

	pub fn insert(&mut self, pid: Pid) {
		self.members.insert(pid);
	}

	pub fn remove(&mut self, pid: &Pid) {
		self.members.remove(pid);
	}

	pub fn get(&self, pid: &Pid) -> Option<&Pid> {
		self.members.get(pid)
	}

	pub fn is_empty(&self) -> bool {
		self.members.is_empty()
	}
}

type Shared<T> = Arc<Locked<T>>;

pub struct ProcessGroupTree(BTreeMap<Pgid, Shared<ProcessGroup>>);

impl ProcessGroupTree {
	pub const fn new() -> Self {
		Self(BTreeMap::new())
	}

	pub fn get_or_insert(&mut self, pgid: Pgid) -> &Shared<ProcessGroup> {
		self.0
			.entry(pgid)
			.or_insert_with(|| Arc::new(Locked::new(ProcessGroup::new(pgid))))
	}

	pub fn remove(&mut self, pgid: &Pgid) {
		self.0.remove(pgid);
	}

	pub fn get(&self, pgid: &Pgid) -> Option<&Shared<ProcessGroup>> {
		self.0.get(pgid)
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

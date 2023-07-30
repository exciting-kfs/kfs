use alloc::collections::{
	btree_map::{self, Entry},
	BTreeMap, BTreeSet,
};

use crate::process::{
	exit::ExitStatus,
	relation::{Pgid, Pid},
};

#[derive(Clone, Copy)]
pub struct Zombie {
	pub pid: Pid,
	pub pgid: Pgid,
	pub exit_status: ExitStatus,
}

impl Zombie {
	pub fn new(pid: Pid, pgid: Pgid, status: ExitStatus) -> Self {
		Self {
			pid,
			pgid,
			exit_status: status,
		}
	}
}

pub struct ZombieMap {
	by_pid: BTreeMap<Pid, Zombie>,
	by_pgid: BTreeMap<Pgid, BTreeSet<Pid>>,
}

impl ZombieMap {
	pub const fn new() -> Self {
		Self {
			by_pid: BTreeMap::new(),
			by_pgid: BTreeMap::new(),
		}
	}

	pub fn insert(&mut self, zombie: Zombie) {
		self.by_pid.insert(zombie.pid, zombie);

		self.by_pgid
			.entry(zombie.pgid)
			.or_insert_with(|| BTreeSet::new())
			.insert(zombie.pid);
	}

	pub fn iter(&self) -> btree_map::Iter<'_, Pid, Zombie> {
		self.by_pid.iter()
	}

	pub fn remove_by_pid(&mut self, pid: Pid) -> Option<Zombie> {
		let zombie = self.remove_from_pid(pid)?;

		self.remove_from_pgid(zombie.pgid, Some(pid))
			.expect("inconsitent tree");

		Some(zombie)
	}

	pub fn remove_by_pgid(&mut self, pgid: Pgid) -> Option<Zombie> {
		let zombie_pid = self.remove_from_pgid(pgid, None)?;

		self.remove_from_pid(zombie_pid)
	}

	pub fn remove_by_any(&mut self) -> Option<Zombie> {
		let (_, zombie) = self.by_pid.pop_first()?;

		self.remove_from_pgid(zombie.pgid, Some(zombie.pid))
			.expect("inconsistent tree");

		Some(zombie)
	}

	fn remove_from_pid(&mut self, pid: Pid) -> Option<Zombie> {
		self.by_pid.remove(&pid)
	}

	fn remove_from_pgid(&mut self, pgid: Pgid, pid: Option<Pid>) -> Option<Pid> {
		let pgroup_entry = self.by_pgid.entry(pgid);

		if let Entry::Occupied(mut o) = pgroup_entry {
			let pgroup = o.get_mut();

			let zombie_pid = match pid {
				Some(ref x) => pgroup.remove(x).then(|| *x),
				None => pgroup.pop_first(),
			};

			if pgroup.is_empty() {
				o.remove();
			}

			zombie_pid
		} else {
			None
		}
	}
}

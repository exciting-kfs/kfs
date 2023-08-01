pub mod zombie;

use core::mem;

use alloc::collections::BTreeSet;

use crate::process::get_init_task;
use crate::{interrupt::syscall::errno::Errno, process::process_tree::PROCESS_TREE};

use self::zombie::{Zombie, ZombieMap};
use super::{Pgid, Pid};

pub struct Family {
	parent: Pid,
	children: BTreeSet<Pid>,
	zombie: ZombieMap,
}

impl Family {
	pub fn new(parent: Pid) -> Self {
		Self {
			parent,
			children: BTreeSet::new(),
			zombie: ZombieMap::new(),
		}
	}

	pub fn get_ppid(&self) -> Pid {
		self.parent
	}

	pub fn insert_child(&mut self, child: Pid) {
		self.children.insert(child);
	}

	pub fn update_child_to_zombie(&mut self, zombie: Zombie) {
		self.remove_child(zombie.pid);
		self.insert_zombie(zombie);
	}

	pub fn update_parent_to_init(&mut self) {
		self.parent = Pid::from_raw(1);
	}

	fn remove_child(&mut self, pid: Pid) {
		self.children.remove(&pid);
	}

	fn insert_zombie(&mut self, zombie: Zombie) {
		self.zombie.insert(zombie);
	}

	pub fn exit(&mut self, zombie: Zombie) {
		let ppid = self.get_ppid();

		let ptree = PROCESS_TREE.lock();
		let parent = ptree.get(&ppid).expect("invalid ppid");

		let mut parent_relation = parent
			.get_user_ext()
			.expect("parent must be user process")
			.lock_relation();
		parent_relation.update_child_to_zombie(zombie);
		mem::drop(parent_relation);

		let init_task = get_init_task();
		let mut init_relation = init_task.get_user_ext().unwrap().lock_relation();
		for cpid in &self.children {
			let child = ptree.get(&cpid).expect("invalid cpid");

			child
				.get_user_ext()
				.unwrap()
				.lock_relation()
				.update_parent_to_init();
			init_relation.family.insert_child(child.get_pid());
		}

		for (_, zombie) in self.zombie.iter() {
			init_relation.family.insert_zombie(*zombie);
		}
	}

	pub fn wait_any(&mut self) -> Result<Zombie, Errno> {
		self.zombie.remove_by_any().ok_or_else(|| Errno::ECHILD)
	}

	pub fn wait_pid(&mut self, pid: Pid) -> Result<Zombie, Errno> {
		self.zombie.remove_by_pid(pid).ok_or_else(|| Errno::ECHILD)
	}

	pub fn wait_pgid(&mut self, pgid: Pgid) -> Result<Zombie, Errno> {
		self.zombie
			.remove_by_pgid(pgid)
			.ok_or_else(|| Errno::ECHILD)
	}

	pub fn has_child(&self, pid: Pid) -> bool {
		self.children.contains(&pid)
	}
}

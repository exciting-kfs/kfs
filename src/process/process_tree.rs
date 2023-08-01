use alloc::{collections::BTreeMap, sync::Arc};

use crate::sync::locked::Locked;

use super::{relation::Pid, task::Task};

pub struct ProcessTree(BTreeMap<Pid, Arc<Task>>);
pub static PROCESS_TREE: Locked<ProcessTree> = Locked::new(ProcessTree::new());

impl ProcessTree {
	pub const fn new() -> Self {
		Self(BTreeMap::new())
	}

	pub fn members(&self) -> &BTreeMap<Pid, Arc<Task>> {
		&self.0
	}

	pub fn insert(&mut self, task: Arc<Task>) {
		self.0.insert(task.get_pid(), task);
	}

	pub fn remove(&mut self, pid: &Pid) {
		self.0.remove(pid);
	}

	pub fn get(&self, pid: &Pid) -> Option<&Arc<Task>> {
		self.0.get(pid)
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl Locked<ProcessTree> {
	pub fn get_task(&self, pid: Pid) -> Option<Arc<Task>> {
		self.lock().get(&pid).map(|t| t.clone())
	}
}

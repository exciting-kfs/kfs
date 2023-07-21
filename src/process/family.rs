use alloc::collections::{BTreeMap, BTreeSet};

use crate::sync::singleton::Singleton;

use super::pid::{Pid, ReservedPid};

#[derive(Debug)]
#[repr(transparent)]
pub struct ExitStatus {
	pub raw: usize,
}

pub static TASK_TREE: Singleton<TaskTree> = Singleton::new(TaskTree::new());

pub struct TaskTree {
	tree: BTreeMap<Pid, TaskFamily>,
}

impl TaskTree {
	pub const fn new() -> Self {
		Self {
			tree: BTreeMap::new(),
		}
	}

	fn add_task(&mut self, pid: Pid, ppid: Pid) {
		self.tree.insert(
			pid,
			TaskFamily {
				ppid,
				running_children: BTreeSet::new(),
				exited_children: BTreeMap::new(),
			},
		);
	}

	fn get_node(&mut self, pid: Pid) -> &mut TaskFamily {
		self.tree.get_mut(&pid).expect("invalid pid")
	}

	pub fn add_child_task(&mut self, ppid: Pid) -> Pid {
		let pid = Pid::allocate();

		self.add_task(pid, ppid);

		let parent = self.get_node(ppid);

		parent.add_child(pid);

		pid
	}

	pub fn add_init_task(&mut self) -> Pid {
		let pid = Pid::reserved(ReservedPid::Init);
		let ppid = Pid::reserved(ReservedPid::Idle);

		self.add_task(pid, ppid);

		pid
	}

	pub fn exit_task(&mut self, pid: Pid, status: ExitStatus) {
		let me = self.tree.remove(&pid).expect("invalid pid");

		let parent = self.get_node(me.ppid);

		parent.remove_child(pid, status);

		for cpid in me.running_children {
			let child = self.get_node(cpid);
			child.ppid = Pid::reserved(ReservedPid::Init);
		}
	}

	pub fn try_wait_task(&mut self, pid: Pid, cpid: Option<Pid>) -> Result<(Pid, ExitStatus), ()> {
		let me = self.get_node(pid);

		let result = match cpid {
			Some(cpid) => match me.exited_children.remove(&cpid) {
				Some(status) => Ok((cpid, status)),
				None => Err(()),
			},
			None => match me.exited_children.pop_first() {
				Some(x) => Ok(x),
				None => Err(()),
			},
		};

		if let Ok((cpid, _)) = result {
			Pid::deallocate(cpid);
		};

		result
	}
}

#[derive(Debug)]
pub struct TaskFamily {
	ppid: Pid,
	running_children: BTreeSet<Pid>,
	exited_children: BTreeMap<Pid, ExitStatus>,
}

impl TaskFamily {
	fn add_child(&mut self, pid: Pid) {
		self.running_children.insert(pid);
	}

	fn remove_child(&mut self, pid: Pid, status: ExitStatus) {
		self.running_children.remove(&pid);
		self.exited_children.insert(pid, status);
	}
}

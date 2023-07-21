use core::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use alloc::collections::LinkedList;

use crate::sync::singleton::Singleton;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Pid(usize);

pub enum ReservedPid {
	Idle = 0,
	Init = 1,
}

static NEXT_PID: AtomicUsize = AtomicUsize::new(2);
static FREED_PID: Singleton<LinkedList<Pid>> = Singleton::new(LinkedList::new());

impl Pid {
	pub fn allocate() -> Self {
		if let Some(pid) = FREED_PID.lock().pop_front() {
			return pid;
		}

		let pid = NEXT_PID.fetch_add(1, Relaxed);

		Pid(pid)
	}

	pub fn deallocate(self) {
		FREED_PID.lock().push_back(self);
	}

	pub fn reserved(who: ReservedPid) -> Self {
		Pid(who as usize)
	}

	pub fn as_raw(&self) -> usize {
		self.0
	}

	pub fn from_raw(raw: usize) -> Self {
		Pid(raw)
	}
}

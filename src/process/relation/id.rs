use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::collections::LinkedList;

use crate::sync::locked::Locked;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Pid(usize);

static NEXT_PID: AtomicUsize = AtomicUsize::new(2);
static FREED_PID: Locked<LinkedList<Pid>> = Locked::new(LinkedList::new());

impl Pid {
	pub fn allocate() -> Self {
		if let Some(pid) = FREED_PID.lock().pop_front() {
			return pid;
		}

		let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed);

		Pid(pid)
	}

	pub fn deallocate(self) {
		FREED_PID.lock().push_back(self);
	}

	pub fn as_raw(&self) -> usize {
		self.0
	}

	pub fn from_raw(raw: usize) -> Self {
		Pid(raw)
	}
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Pgid(usize);

impl Pgid {
	pub fn new(pid: Pid) -> Self {
		Pgid(pid.as_raw())
	}

	pub fn as_raw(&self) -> usize {
		self.0
	}

	pub fn from_raw(raw: usize) -> Self {
		Pgid(raw)
	}
}

impl Default for Pgid {
	fn default() -> Self {
		Self::from_raw(0)
	}
}

impl From<Pid> for Pgid {
	fn from(value: Pid) -> Self {
		Self::from_raw(value.as_raw())
	}
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Sid(usize);

impl Sid {
	pub fn new(pid: Pid) -> Self {
		Sid(pid.as_raw())
	}

	pub fn as_raw(&self) -> usize {
		self.0
	}

	pub fn from_raw(raw: usize) -> Self {
		Sid(raw)
	}
}

impl Default for Sid {
	fn default() -> Self {
		Self::from_raw(0) // 0?
	}
}

impl From<Pid> for Sid {
	fn from(value: Pid) -> Self {
		Self::from_raw(value.as_raw())
	}
}

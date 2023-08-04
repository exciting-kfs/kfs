use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::collections::BTreeSet;

use crate::{pr_debug, sync::locked::Locked};

static PID_ALLOC: Locked<PidAlloc> = Locked::new(PidAlloc::new());

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Pid(usize);

impl Pid {
	pub fn allocate() -> Self {
		PID_ALLOC.lock().alloc_pid()
	}

	pub fn deallocate(self) {
		PID_ALLOC.lock().dealloc_pid(self)
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

	pub fn deallocate(self) {
		PID_ALLOC.lock().dealloc_pgid(self)
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

struct PidAlloc {
	end: AtomicUsize,
	allocatable: BTreeSet<usize>,
	free_pid: BTreeSet<usize>,
	free_pgid: BTreeSet<usize>,
}

impl PidAlloc {
	pub const fn new() -> Self {
		Self {
			end: AtomicUsize::new(2),
			allocatable: BTreeSet::new(),
			free_pid: BTreeSet::new(),
			free_pgid: BTreeSet::new(),
		}
	}

	/// the `pgid` equal to `pid` is also allocated implicitly.
	pub fn alloc_pid(&mut self) -> Pid {
		let pid = match self.allocatable.pop_first() {
			Some(s) => s,
			None => self.end.fetch_add(1, Ordering::Relaxed),
		};

		Pid(pid)
	}

	pub fn dealloc_pid(&mut self, pid: Pid) {
		pr_debug!("DEALLOC: {:?}", pid);

		let pid = pid.as_raw();
		let end = self.end.load(Ordering::Relaxed);

		debug_assert!(pid < end, "invalid pgid deallocation.");
		debug_assert!(
			match self.free_pgid.remove(&pid) {
				true => self.allocatable.insert(pid),
				false => self.free_pid.insert(pid),
			},
			"invalid pid deallcation"
		);
	}

	pub fn dealloc_pgid(&mut self, pgid: Pgid) {
		pr_debug!("DEALLOC: {:?}", pgid);
		let pgid = pgid.as_raw();
		let end = self.end.load(Ordering::Relaxed);

		debug_assert!(pgid < end, "invalid pgid deallocation.");
		match self.free_pid.remove(&pgid) {
			true => self.allocatable.insert(pgid),
			false => self.free_pgid.insert(pgid),
		};
	}

	pub fn stat(&self) -> PidAllocStat {
		PidAllocStat {
			end: self.end.load(Ordering::Relaxed) as isize,
			allocatable_cnt: self.allocatable.iter().count() as isize,
			free_pid_cnt: self.free_pid.iter().count() as isize,
			free_pgid_cnt: self.free_pgid.iter().count() as isize,
		}
	}
}

#[derive(PartialEq, Eq, Debug)]
struct PidAllocStat {
	end: isize,
	allocatable_cnt: isize,
	free_pid_cnt: isize,
	free_pgid_cnt: isize,
}

impl PidAllocStat {
	fn hand_made(
		end: isize,
		allocatable_cnt: isize,
		free_pid_cnt: isize,
		free_pgid_cnt: isize,
	) -> Self {
		Self {
			end,
			allocatable_cnt,
			free_pid_cnt,
			free_pgid_cnt,
		}
	}

	fn delta(
		&self,
		end: isize,
		allocatable_cnt: isize,
		free_pid_cnt: isize,
		free_pgid_cnt: isize,
	) -> Self {
		Self {
			end: self.end + end,
			allocatable_cnt: self.allocatable_cnt + allocatable_cnt,
			free_pid_cnt: self.free_pid_cnt + free_pid_cnt,
			free_pgid_cnt: self.free_pgid_cnt + free_pgid_cnt,
		}
	}
}

mod test {
	use super::*;
	use kfs_macro::ktest;

	#[ktest(pid)]
	fn test_allocate() {
		let mut alloc = PidAlloc::new();
		let prev = alloc.stat();
		let end = prev.end as usize;

		assert_eq!(Pid(end), alloc.alloc_pid());
		assert_eq!(Pid(end + 1), alloc.alloc_pid());
		assert_eq!(Pid(end + 2), alloc.alloc_pid());

		assert_eq!(alloc.stat(), prev.delta(3, 0, 0, 0));
	}

	#[ktest(pid)]
	fn test_pid_deallocate() {
		let mut alloc = PidAlloc::new();
		let prev = alloc.stat();
		let end = prev.end as usize;

		assert_eq!(Pid(end), alloc.alloc_pid());
		assert_eq!(Pid(end + 1), alloc.alloc_pid());
		assert_eq!(Pid(end + 2), alloc.alloc_pid());

		alloc.dealloc_pid(Pid(end));
		alloc.dealloc_pid(Pid(end + 1));
		alloc.dealloc_pid(Pid(end + 2));

		assert_eq!(alloc.stat(), prev.delta(3, 0, 3, 0));
	}

	#[ktest(pid)]
	fn test_pgid_deallocate() {
		let mut alloc = PidAlloc::new();
		let prev = alloc.stat();
		let end = prev.end as usize;

		assert_eq!(Pid(end), alloc.alloc_pid());
		assert_eq!(Pid(end + 1), alloc.alloc_pid());
		assert_eq!(Pid(end + 2), alloc.alloc_pid());

		alloc.dealloc_pgid(Pgid(end));
		alloc.dealloc_pgid(Pgid(end + 1));
		alloc.dealloc_pgid(Pgid(end + 2));

		assert_eq!(alloc.stat(), prev.delta(3, 0, 0, 3));

		alloc.dealloc_pid(Pid(end));
		alloc.dealloc_pid(Pid(end + 1));
		alloc.dealloc_pid(Pid(end + 2));
		assert_eq!(alloc.stat(), prev.delta(3, 3, 0, 0));
	}

	#[ktest(pid)]
	fn test_reallocate() {
		let mut alloc = PidAlloc::new();
		let prev = alloc.stat();
		let end = prev.end as usize;

		assert_eq!(Pid(end), alloc.alloc_pid());

		alloc.dealloc_pgid(Pgid(end));
		alloc.dealloc_pid(Pid(end));
		assert_eq!(alloc.stat(), prev.delta(1, 1, 0, 0));

		assert_eq!(Pid(end), alloc.alloc_pid());
		assert_eq!(alloc.stat(), prev.delta(1, 0, 0, 0));
	}
}

use core::sync::atomic::{AtomicUsize, Ordering};

use alloc::collections::{BTreeMap, BTreeSet};

use crate::{pr_debug, sync::Locked};

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

	pub const fn from_raw(raw: usize) -> Self {
		Pid(raw)
	}
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Pgid(usize);

impl Pgid {
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
		Pgid(0)
	}
}

impl From<Pid> for Pgid {
	fn from(pid: Pid) -> Self {
		let id = pid.as_raw();
		PID_ALLOC.lock().alloc_id(id, IdKind::Pgid);
		Pgid(id)
	}
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Sid(usize);

impl Sid {
	pub fn as_raw(&self) -> usize {
		self.0
	}

	pub fn from_raw(raw: usize) -> Self {
		Sid(raw)
	}

	pub fn deallocate(self) {
		PID_ALLOC.lock().dealloc_sid(self)
	}
}

impl Default for Sid {
	fn default() -> Self {
		Sid(0)
	}
}

impl From<Pid> for Sid {
	fn from(pid: Pid) -> Self {
		let id = pid.as_raw();
		PID_ALLOC.lock().alloc_id(id, IdKind::Sid);
		Sid(id)
	}
}

struct PidAlloc {
	end: AtomicUsize,
	free: BTreeSet<usize>,
	allocated: BTreeMap<usize, [bool; 3]>,
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
enum IdKind {
	Pid,
	Pgid,
	Sid,
}

impl PidAlloc {
	pub const fn new() -> Self {
		Self {
			end: AtomicUsize::new(0),
			free: BTreeSet::new(),
			allocated: BTreeMap::new(),
		}
	}

	pub fn alloc_pid(&mut self) -> Pid {
		let pid = match self.free.pop_first() {
			Some(s) => s,
			None => self.end.fetch_add(1, Ordering::Relaxed),
		};

		self.allocated.insert(pid, [true, false, false]);
		Pid(pid)
	}

	fn alloc_id(&mut self, id: usize, kind: IdKind) {
		debug_assert!(kind != IdKind::Pid, "invalid id allocation.");
		self.allocated
			.entry(id)
			.and_modify(|info| info[kind as usize] = true);
	}

	fn is_less_than_end(&self, id: usize) -> bool {
		let end = self.end.load(Ordering::Relaxed);
		id < end
	}

	fn free_id(&mut self, id: usize) {
		self.allocated.remove(&id);
		self.free.insert(id);
	}

	fn dealloc_id(&mut self, id: usize, kind: IdKind) {
		debug_assert!(
			self.is_less_than_end(id),
			"invalid {:?} deallocation.",
			kind
		);

		let info = self.allocated.get_mut(&id).expect("allocation info");
		info[kind as usize] = false;

		if info.iter().filter(|e| **e).count() == 0 {
			self.free_id(id);
		}
	}

	pub fn dealloc_pid(&mut self, pid: Pid) {
		pr_debug!("DEALLOC: {:?}", pid);
		self.dealloc_id(pid.as_raw(), IdKind::Pid);
	}

	pub fn dealloc_pgid(&mut self, pgid: Pgid) {
		pr_debug!("DEALLOC: {:?}", pgid);
		self.dealloc_id(pgid.as_raw(), IdKind::Pgid);
	}

	pub fn dealloc_sid(&mut self, sid: Sid) {
		pr_debug!("DEALLOC: {:?}", sid);
		self.dealloc_id(sid.as_raw(), IdKind::Sid);
	}

	fn stat(&self) -> PidAllocStat {
		PidAllocStat {
			end: self.end.load(Ordering::Relaxed) as isize,
			allocated_cnt: self.allocated.iter().count() as isize,
			free_cnt: self.free.iter().count() as isize,
		}
	}
}

#[derive(PartialEq, Eq, Debug)]
struct PidAllocStat {
	end: isize,
	allocated_cnt: isize,
	free_cnt: isize,
}

impl PidAllocStat {
	fn hand_made(end: isize, allocated_cnt: isize, free_cnt: isize) -> Self {
		Self {
			end,
			allocated_cnt,
			free_cnt,
		}
	}
}

mod test {

	use super::*;
	use kfs_macro::ktest;

	#[ktest(pid_alloc)]
	fn test_alloc_pid() {
		let mut pa = PidAlloc::new();

		assert_eq!(pa.alloc_pid(), Pid::from_raw(0));
		assert_eq!(pa.alloc_pid(), Pid::from_raw(1));

		assert_eq!(*pa.allocated.get(&0).unwrap(), [true, false, false]);
		assert_eq!(*pa.allocated.get(&1).unwrap(), [true, false, false]);

		assert_eq!(pa.stat(), PidAllocStat::hand_made(2, 2, 0));
	}

	#[ktest(pid_alloc)]
	fn test_dealloc_pid() {
		let mut pa = PidAlloc::new();

		assert_eq!(pa.alloc_pid(), Pid::from_raw(0));
		assert_eq!(*pa.allocated.get(&0).unwrap(), [true, false, false]);
		pa.dealloc_pid(Pid::from_raw(0));

		assert_eq!(pa.stat(), PidAllocStat::hand_made(1, 0, 1));
	}

	#[ktest(pid_alloc)]
	fn test_pgid() {
		let pid = PID_ALLOC.lock().alloc_pid();
		let pgid = Pgid::from(pid);

		let mut pa = PID_ALLOC.lock();
		assert_eq!(
			*pa.allocated.get(&pgid.as_raw()).unwrap(),
			[true, true, false]
		);

		pa.dealloc_pid(pid);

		assert_eq!(
			*pa.allocated.get(&pgid.as_raw()).unwrap(),
			[false, true, false]
		);

		pa.dealloc_pgid(pgid);

		pa.free.get(&pgid.as_raw()).expect("id freed");
	}

	#[ktest(pid_alloc)]
	fn test_sid() {
		let pid = PID_ALLOC.lock().alloc_pid();
		let sid = Sid::from(pid);

		let mut pa = PID_ALLOC.lock();
		assert_eq!(
			*pa.allocated.get(&sid.as_raw()).unwrap(),
			[true, false, true]
		);

		pa.dealloc_pid(pid);

		assert_eq!(
			*pa.allocated.get(&sid.as_raw()).unwrap(),
			[false, false, true]
		);

		pa.dealloc_sid(sid);

		pa.free.get(&sid.as_raw()).expect("id freed");
	}

	#[ktest(pid_alloc)]
	fn test_realloc() {
		let mut pa = PID_ALLOC.lock();
		let pid = pa.alloc_pid();
		pa.dealloc_pid(pid);
		pa.free.get(&pid.as_raw()).expect("id freed");

		let pid2 = pa.alloc_pid();

		assert_eq!(pid, pid2);
	}
}

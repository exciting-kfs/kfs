use core::{alloc::AllocError, cmp::Ordering, ops::Range};

use alloc::vec::Vec;
use bitflags::bitflags;

use super::constant::{PAGE_SIZE, PT_COVER_SIZE, VM_OFFSET};

bitflags! {
	#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
	pub struct AreaFlag: u32 {
		const Readable = (1 << 0);
		const Writable = (1 << 1);
	}
}

/// user memroy area.
/// area is half-opened [start, end)
pub struct Area {
	pub start: usize,
	pub end: usize,
	pub flags: AreaFlag,
}

impl Area {
	pub fn new(start: usize, end: usize, flags: AreaFlag) -> Self {
		Self { start, end, flags }
	}

	pub fn contains(&self, addr: usize) -> bool {
		self.start <= addr && addr < self.end
	}

	pub fn is_overlap(&self, other: &Area) -> bool {
		other.start < self.end || self.start < other.end
	}

	/// compare self with address.
	/// # Example
	/// - `S . E` => Equal
	/// - `. S E` => Greater
	/// - `S E .` => Less
	pub fn cmp_addr(&self, addr: usize) -> Ordering {
		if self.end <= addr {
			Ordering::Less
		} else if self.start > addr {
			Ordering::Greater
		} else {
			Ordering::Equal
		}
	}
}

const VALID_USER_AREA: Range<usize> = PT_COVER_SIZE..VM_OFFSET;

pub struct UserAddressSpace {
	areas: Vec<Area>,
}

impl UserAddressSpace {
	pub fn new() -> Self {
		Self { areas: Vec::new() }
	}

	pub fn find_area(&self, addr: usize) -> Option<&Area> {
		self.areas
			.binary_search_by(|x| x.cmp_addr(addr))
			.ok()
			.map(|idx| &self.areas[idx])
	}

	pub fn query_flag(&self, addr: usize, flag: AreaFlag) -> bool {
		self.find_area(addr)
			.is_some_and(|area| area.flags.contains(flag))
	}

	pub fn allocate_fixed_area(
		&mut self,
		start: usize,
		count: usize,
		flags: AreaFlag,
	) -> Result<usize, AllocError> {
		let end = count
			.checked_mul(PAGE_SIZE)
			.and_then(|x| x.checked_add(start))
			.ok_or(AllocError)?;

		if start < VALID_USER_AREA.start || VALID_USER_AREA.end <= end {
			return Err(AllocError);
		}

		let new_area = Area::new(start, end, flags);

		let idx = self.areas.partition_point(|x| new_area.end <= x.start);

		if idx == 0 || !self.areas[idx - 1].is_overlap(&new_area) {
			self.areas.insert(idx, new_area)
		}

		Ok(start)
	}

	pub fn allocate_area(&mut self, count: usize, flags: AreaFlag) -> Result<usize, AllocError> {
		let size = count.checked_mul(PAGE_SIZE).ok_or(AllocError)?;

		let l = Some(VALID_USER_AREA.start)
			.iter()
			.chain(self.areas.iter().map(|area| &area.end));

		let r = self
			.areas
			.iter()
			.map(|area| &area.start)
			.chain(Some(VALID_USER_AREA.end).iter());

		let (idx, (start, _)) = l
			.zip(r)
			.enumerate()
			.find(|(_, (start, end))| *end - *start >= size)
			.ok_or(AllocError)?;

		let start = *start;

		self.areas
			.insert(idx, Area::new(start, start + PAGE_SIZE * count, flags));

		Ok(start)
	}

	pub fn deallocate_area(&mut self, area_start: usize) -> Option<Area> {
		let idx = self
			.areas
			.binary_search_by(|x| x.start.cmp(&area_start))
			.ok()?;

		Some(self.areas.remove(idx))
	}
}

mod test {
	use crate::{
		mm::constant::{LAST_PFN, MB},
		pr_info,
	};

	use super::*;
	use kfs_macro::ktest;

	#[ktest(uvma)]
	fn alloc_query() {
		let mut us = UserAddressSpace::new();

		let start = us.allocate_area(10, AreaFlag::Readable).unwrap();

		assert!(us.query_flag(start, AreaFlag::Readable));
		assert!(us.query_flag((start + PAGE_SIZE * 10) - 1, AreaFlag::Readable));
		assert!(!us.query_flag(start + PAGE_SIZE * 10, AreaFlag::Readable));
	}

	#[ktest(uvma)]
	fn alloc_dealloc() {
		let mut us = UserAddressSpace::new();

		let start = us.allocate_area(10, AreaFlag::Readable).unwrap();
		assert!(us.query_flag(start, AreaFlag::Readable));

		let area = us.deallocate_area(start).unwrap();
		assert!(
			area.start == start
				&& area.end == area.start + PAGE_SIZE * 10
				&& area.flags == AreaFlag::Readable
		);
		assert!(!us.query_flag(start, AreaFlag::Readable));
	}

	#[ktest(uvma)]
	fn fixed_alloc() {
		let mut us = UserAddressSpace::new();

		// null area cannot be allocated.
		assert!(matches!(
			us.allocate_fixed_area(0, 1, AreaFlag::Readable),
			Err(_)
		));

		// kernel area cannot be allocated.
		assert!(matches!(
			us.allocate_fixed_area(VALID_USER_AREA.end, 1, AreaFlag::Readable),
			Err(_)
		));

		let begin = us
			.allocate_fixed_area(VALID_USER_AREA.start, 1, AreaFlag::Readable)
			.unwrap();
		assert!(begin == VALID_USER_AREA.start);
		assert!(us.query_flag(begin, AreaFlag::Readable));
		assert!(us.query_flag(begin + PAGE_SIZE - 1, AreaFlag::Readable));
		assert!(!us.query_flag(begin + PAGE_SIZE, AreaFlag::Readable));
	}

	#[ktest(uvma)]
	fn full_area() {
		let mut us = UserAddressSpace::new();

		let mut count = LAST_PFN;
		let start = loop {
			if let Ok(s) = us.allocate_area(count, AreaFlag::Readable) {
				break s;
			}
			count -= 1;
		};
		let end = start + count * PAGE_SIZE;

		pr_info!(
			" note: {:#010x}..{:#010x} ({} MB) reserved by user.",
			start,
			end,
			(count * PAGE_SIZE) / MB,
		);
		assert!(VALID_USER_AREA.start <= start && end <= VALID_USER_AREA.end);

		assert!(us.query_flag(start, AreaFlag::Readable));
		us.deallocate_area(start);
		assert!(!us.query_flag(start, AreaFlag::Readable));

		let area1 = us.allocate_area(count - 1, AreaFlag::Readable).unwrap();
		let area2 = us.allocate_area(1, AreaFlag::Readable).unwrap();
		assert!(us.query_flag(area1, AreaFlag::Readable));
		assert!(us.query_flag(area2, AreaFlag::Readable));
	}
}

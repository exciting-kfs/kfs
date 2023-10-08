use core::{alloc::AllocError, cmp::Ordering, ops::Range};

use alloc::vec::Vec;
use bitflags::bitflags;

use crate::mm::{constant::*, page::PageFlag};

bitflags! {
	#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
	pub struct AreaFlag: u32 {
		const Readable = (1 << 0);
		const Writable = (1 << 1);
	}
}

impl Into<PageFlag> for AreaFlag {
	fn into(self) -> PageFlag {
		if self.contains(AreaFlag::Writable) {
			PageFlag::USER_RDWR
		} else {
			PageFlag::USER_RDONLY
		}
	}
}

/// user memroy area.
/// area is half-opened [start, end)
#[derive(Clone)]
pub struct Area {
	pub start: usize,
	pub end: usize,
	pub flags: AreaFlag,
}

pub struct AreaPageIter {
	start: usize,
	nr_pages: usize,
}

impl Iterator for AreaPageIter {
	type Item = usize;

	fn next(&mut self) -> Option<Self::Item> {
		if self.nr_pages == 0 {
			return None;
		}

		let ret = self.start;

		self.start += PAGE_SIZE;
		self.nr_pages -= 1;

		Some(ret)
	}
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

	pub fn iter_pages(&self) -> AreaPageIter {
		let nr_pages = (self.end - self.start) / PAGE_SIZE;

		AreaPageIter {
			start: self.start,
			nr_pages,
		}
	}
}

const VALID_USER_AREA: Range<usize> = PT_COVER_SIZE..VM_OFFSET;

#[derive(Clone)]
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

		let r_area_idx = match self.areas.binary_search_by_key(&start, |area| area.start) {
			Ok(_) => Err(AllocError),
			Err(i) => Ok(i),
		}?;

		let l_area_end = if r_area_idx == 0 {
			VALID_USER_AREA.start
		} else {
			self.areas[r_area_idx - 1].end
		};

		let r_area_start = if r_area_idx == self.areas.len() {
			VALID_USER_AREA.end
		} else {
			self.areas[r_area_idx].start
		};

		if l_area_end <= start && end <= r_area_start {
			self.areas.insert(r_area_idx, Area::new(start, end, flags));
			return Ok(start);
		} else {
			return Err(AllocError);
		}
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

	pub fn get_areas(&self) -> &Vec<Area> {
		&self.areas
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

	#[ktest(uvma)]
	fn overlap() {
		let mut us = UserAddressSpace::new();

		us.allocate_fixed_area(0xb000_2000, 2, AreaFlag::Readable)
			.unwrap();
		us.allocate_fixed_area(0xb000_6000, 2, AreaFlag::Readable)
			.unwrap();

		let temp = us
			.allocate_fixed_area(0xb000_0000, 2, AreaFlag::Readable)
			.unwrap();
		us.deallocate_area(temp);

		us.allocate_fixed_area(0xb000_1000, 2, AreaFlag::Readable)
			.unwrap_err();
		us.allocate_fixed_area(0xb000_2000, 2, AreaFlag::Readable)
			.unwrap_err();
		us.allocate_fixed_area(0xb000_3000, 2, AreaFlag::Readable)
			.unwrap_err();

		let temp = us
			.allocate_fixed_area(0xb000_4000, 2, AreaFlag::Readable)
			.unwrap();
		us.deallocate_area(temp);

		us.allocate_fixed_area(0xb000_5000, 2, AreaFlag::Readable)
			.unwrap_err();
		us.allocate_fixed_area(0xb000_6000, 2, AreaFlag::Readable)
			.unwrap_err();
		us.allocate_fixed_area(0xb000_7000, 2, AreaFlag::Readable)
			.unwrap_err();

		let temp = us
			.allocate_fixed_area(0xb000_8000, 2, AreaFlag::Readable)
			.unwrap();
		us.deallocate_area(temp);
	}
}

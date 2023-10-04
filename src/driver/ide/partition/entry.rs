use core::fmt::Display;

use crate::driver::{
	dev_num::DevNum,
	ide::{
		ide_id::{IDE_MAJOR, IDE_MINOR_END},
		lba::LBA28,
	},
};

use super::PartitionType;

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct MaybeEntry(PartitionEntry);

impl MaybeEntry {
	pub fn get_mut(&mut self) -> Option<&mut PartitionEntry> {
		if self.0.partition_type == PartitionType::Empty {
			None
		} else {
			Some(&mut self.0)
		}
	}

	pub fn get(&self) -> Option<&PartitionEntry> {
		if self.0.partition_type == PartitionType::Empty {
			None
		} else {
			Some(&self.0)
		}
	}

	pub const fn empty() -> Self {
		Self(PartitionEntry {
			attribute: 0,
			begin_h: 0,
			begin_s: 0,
			begin_c: 0,
			partition_type: PartitionType::Empty,
			last_h: 0,
			last_s: 0,
			last_c: 0,
			begin_lba: 0,
			sector_count: 0,
		})
	}
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct PartitionEntry {
	attribute: u8,
	begin_h: u8,
	begin_s: u8,
	begin_c: u8,
	pub partition_type: PartitionType,
	last_h: u8,
	last_s: u8,
	last_c: u8,
	begin_lba: u32,
	sector_count: u32,
}

impl PartitionEntry {
	pub fn begin(&self) -> LBA28 {
		unsafe { LBA28::new_unchecked(self.begin_lba as usize) }
	}

	pub fn end(&self) -> LBA28 {
		self.begin() + self.sector_count as usize
	}
}

impl Display for PartitionEntry {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "Partition Entry:\n")?;
		write!(f, "\tattr: {:x},", self.attribute)?;
		write!(f, "\ttype: {:x}\n", self.partition_type)?;
		write!(
			f,
			"\tbegin CHS: ({:x}, {:x}, {:x})\n",
			self.begin_c, self.begin_h, self.begin_s
		)?;
		write!(
			f,
			"\tlast  CHS: ({:x}, {:x}, {:x})\n",
			self.last_c, self.last_h, self.last_s
		)?;
		write!(f, "\tbegin LBA: {:x}\n", self.begin_lba)?;
		write!(f, "\tlast  LBA: {:x}\n", self.end())?;
		write!(f, "\tsector count: {:x}\n", self.sector_count)?;
		Ok(())
	}
}

#[derive(Clone, Copy)]
pub struct EntryIndex(usize);

impl EntryIndex {
	pub fn new(index: usize) -> Option<Self> {
		if index > 4 {
			None
		} else {
			Some(Self(index))
		}
	}

	pub unsafe fn new_unchecked(index: usize) -> Self {
		Self(index)
	}

	pub fn index(&self) -> usize {
		self.0 as usize
	}

	pub fn from_devnum(dev: &DevNum) -> Option<EntryIndex> {
		let remains = dev.minor % IDE_MINOR_END;
		if dev.major == IDE_MAJOR && remains != 0 {
			Self::new(remains - 1)
		} else {
			None
		}
	}
}

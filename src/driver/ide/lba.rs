use core::fmt::LowerHex;

use crate::mm::constant::SECTOR_SIZE;

use super::block::BlockSize;

/// Logical Block Address
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
#[repr(transparent)]
pub struct LBA28(usize);

impl LBA28 {
	const END: u32 = 1 << 28;
	pub fn new(value: usize) -> Option<Self> {
		if value as u32 > Self::END {
			None
		} else {
			Some(Self(value))
		}
	}

	pub unsafe fn new_unchecked(value: usize) -> Self {
		LBA28(value)
	}

	pub unsafe fn from_bytes(bytes: usize) -> Self {
		Self::new_unchecked(bytes / SECTOR_SIZE)
	}

	pub fn end() -> Self {
		LBA28(Self::END as usize)
	}

	pub fn as_raw(&self) -> usize {
		self.0
	}

	pub fn block_size_add(&self, block_size: BlockSize, count: usize) -> Self {
		*self + block_size.as_bytes() / SECTOR_SIZE * count
	}
}

impl LowerHex for LBA28 {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		LowerHex::fmt(&self.0, f)
	}
}

impl core::ops::Add<usize> for LBA28 {
	type Output = LBA28;
	fn add(self, rhs: usize) -> Self::Output {
		if self.0 + rhs >= Self::END as usize {
			LBA28::end()
		} else {
			unsafe { LBA28::new_unchecked(self.0 + rhs) }
		}
	}
}

impl core::ops::Sub<Self> for LBA28 {
	type Output = usize;
	fn sub(self, rhs: Self) -> Self::Output {
		if self.0 >= rhs.0 {
			self.0 - rhs.0
		} else {
			0
		}
	}
}

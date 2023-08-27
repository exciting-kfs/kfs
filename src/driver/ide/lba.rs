use core::fmt::LowerHex;

/// Logical Block Address
#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub struct LBA28(usize);

const LBA28_END: u32 = 1 << 28;

impl LBA28 {
	pub fn new(value: usize) -> Self {
		debug_assert!(value < LBA28_END as usize, "invalid LBA value");
		LBA28(value)
	}

	pub fn end() -> Self {
		LBA28(LBA28_END as usize)
	}

	pub fn as_raw(&self) -> usize {
		debug_assert!(self.0 < LBA28_END as usize, "invalid LBA value");
		self.0
	}

	/// This function only works for CHS in partition table.
	pub fn from_chs(c: u8, h: u8, s: u8) -> Self {
		const HPC: isize = 16;
		const SPT: isize = 63;

		let c = (s as isize & 0xc0 << 8) + c as isize;
		let s = s as isize & 0x3f;
		let h = h as isize;

		LBA28::new(((c * HPC + h) * SPT + (s - 1)) as usize)
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
		if self.0 + rhs >= LBA28_END as usize {
			LBA28::end()
		} else {
			LBA28::new(self.0 + rhs)
		}
	}
}

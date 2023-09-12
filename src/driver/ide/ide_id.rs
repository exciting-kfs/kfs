use crate::driver::dev_num::DevNum;

pub const IDE_MAJOR: usize = 3;
pub const IDE_MINOR_END: usize = 64;
pub const NR_IDE_DEV: usize = 4;

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct IdeId(usize);

impl IdeId {
	pub const fn new(num: usize) -> Option<Self> {
		if num < 4 {
			Some(IdeId(num))
		} else {
			None
		}
	}

	pub const unsafe fn new_unchecked(num: usize) -> Self {
		IdeId(num)
	}

	pub fn from_devnum(dev: &DevNum) -> Option<Self> {
		if dev.major == IDE_MAJOR && dev.minor % IDE_MINOR_END != 0 {
			Self::new(dev.minor / IDE_MINOR_END)
		} else {
			None
		}
	}

	#[inline]
	pub fn channel(&self) -> usize {
		self.0 / 2
	}

	#[inline]
	pub fn index(&self) -> usize {
		self.0
	}

	#[inline]
	pub fn index_in_channel(&self) -> usize {
		self.0 % 2
	}

	#[inline]
	pub fn is_primary(&self) -> bool {
		self.0 % 2 == 0
	}

	#[inline]
	pub fn is_secondary(&self) -> bool {
		self.0 % 2 == 1
	}

	pub fn pair(&self) -> IdeId {
		IdeId(self.0 ^ 0x01)
	}
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct DevNum(usize);

impl DevNum {
	pub const fn new(num: usize) -> Option<Self> {
		if num < 4 {
			Some(DevNum(num))
		} else {
			None
		}
	}

	pub const unsafe fn new_unchecked(num: usize) -> Self {
		DevNum(num)
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

	pub fn pair(&self) -> DevNum {
		DevNum(self.0 ^ 0x01)
	}
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct DevNum(usize);

impl DevNum {
	pub const fn new(num: usize) -> Self {
		debug_assert!(num < 4);
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

	pub fn pair(&self) -> DevNum {
		DevNum(self.0 ^ 0x01)
	}
}

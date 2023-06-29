#[derive(Clone, Copy)]
pub struct BitRange {
	pub start: usize,
	pub end: usize,
}

impl BitRange {
	pub const fn new(start: usize, end: usize) -> Self {
		Self { start, end }
	}

	const fn make_mask(idx: usize) -> usize {
		match 1usize.checked_shl(idx as u32) {
			Some(x) => x,
			None => 0,
		}
		.wrapping_sub(1)
	}

	pub const fn mask(&self) -> usize {
		Self::make_mask(self.end) & !Self::make_mask(self.start)
	}

	pub fn fit(&self, data: usize) -> usize {
		(data << self.start) & self.mask()
	}
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct BitData {
	data: usize,
}

impl BitData {
	pub const fn new(data: usize) -> Self {
		Self { data }
	}

	pub fn erase_bits(&mut self, range: &BitRange) -> &mut Self {
		self.data &= !range.mask();

		self
	}

	pub fn shift_add_bits(&mut self, range: &BitRange, data: usize) -> &mut Self {
		self.data |= range.fit(data);

		self
	}

	pub fn add_bits(&mut self, range: &BitRange, data: usize) -> &mut Self {
		self.data |= data & range.mask();

		self
	}

	pub fn get_raw_bits(&self) -> usize {
		self.data
	}

	pub fn get_bits(&self, range: &BitRange) -> usize {
		self.data & range.mask()
	}

	pub fn shift_get_bits(&self, range: &BitRange) -> usize {
		(self.data & range.mask()) >> range.start
	}
}

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
}

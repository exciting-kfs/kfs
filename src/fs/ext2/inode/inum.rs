#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct Inum(usize);

impl Inum {
	pub fn new(num: usize) -> Option<Inum> {
		if num >= 1 {
			Some(Inum(num))
		} else {
			None
		}
	}

	pub unsafe fn new_unchecked(num: usize) -> Inum {
		Inum(num)
	}

	#[inline]
	pub fn index(&self) -> usize {
		self.0 - 1
	}

	pub fn ino(&self) -> usize {
		self.0
	}
}

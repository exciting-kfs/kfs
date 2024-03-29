/// Classic pseudo random number generator
/// with LCG (Linear congruential generator) implementation.
pub struct LCG {
	context: u32,
}

impl LCG {
	pub fn new(seed: u32) -> Self {
		LCG { context: seed }
	}

	pub fn rand(&mut self) -> u32 {
		// magic number was copied from glibc's rand(3).
		self.context = self.context.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;

		return self.context;
	}
}

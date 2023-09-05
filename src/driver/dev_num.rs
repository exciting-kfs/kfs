#[derive(Clone, Debug)]
pub struct DevNum {
	pub major: usize,
	pub minor: usize,
}

impl DevNum {
	pub fn new(major: usize, minor: usize) -> Self {
		Self { major, minor }
	}
}

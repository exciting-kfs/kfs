//! Console Cursor

pub type Result<T> = core::result::Result<T, ()>; // TODO 이거 별로 맘에 안드는데..

#[derive(Clone, Copy)]
pub struct Cursor<const H: usize, const W: usize> {
	y: isize,
	x: isize,
}

impl<const HEIGHT: usize, const WIDTH: usize> Cursor<HEIGHT, WIDTH> {
	const HEIGHT: isize = HEIGHT as isize;
	const WIDTH: isize = WIDTH as isize;
	const NEWLINE: isize = Self::WIDTH - 1;

	/// construct new cursor pointing at (y, x)
	pub fn at(y: isize, x: isize) -> Result<Self> {
		if !Self::is_valid(y, x) {
			return Err(());
		}

		let mut result = Self { y, x };
		result.regularize();

		Ok(result)
	}

	pub fn new() -> Self {
		Cursor { y: 0, x: 0 }
	}

	pub const unsafe fn at_unchecked(y: isize, x: isize) -> Self {
		Cursor { y, x }
	}

	/// check `(y, x)` is `regular`.
	fn is_regular(y: isize, x: isize) -> bool {
		let x_ok = 0 <= x && x < Self::WIDTH;
		let y_ok = 0 <= y && y < Self::HEIGHT;

		x_ok && y_ok
	}

	/// check `(y, x)` is `valid`.
	fn is_valid(y: isize, x: isize) -> bool {
		let flat = y * Self::WIDTH + x;

		0 <= flat && flat < Self::WIDTH * Self::HEIGHT
	}

	fn do_move(&mut self, y: isize, x: isize) {
		self.x = x;
		self.y = y;
	}

	/// move cursor relatively.
	pub fn move_rel_y(&mut self, dy: isize) {
		self.y = (self.y + dy).clamp(0, Self::HEIGHT - 1);
	}

	pub fn move_rel_x(&mut self, dx: isize) {
		self.x = (self.x + dx).clamp(0, Self::WIDTH - 1);
	}

	/// convert `valid`, but not `regular` coordinate into `regular` coordinate.
	///
	/// where:
	/// - valid coord: 0 <= (y * WIDTH + x) < WIDTH * HEIGHT
	/// - regular coord: 0 <= y < HEIGHT && 0 <= x < WIDTH
	///
	/// note:
	/// - `regular` coord is `valid` coord.
	/// - but `valid` coord may not `regular` coord.
	fn regularize(&mut self) {
		self.y += self.x / Self::WIDTH;
		self.x %= Self::WIDTH;

		if self.x < 0 {
			self.y -= 1;
			self.x += Self::WIDTH;
		}
	}

	pub fn move_rel_wrap_x(&mut self, dx: isize) {
		self.x += dx;

		let flat = self.y * Self::WIDTH + self.x;
		if flat < 0 {
			self.do_move(0, 0);
			return;
		}

		if flat >= Self::WIDTH * Self::HEIGHT {
			self.do_move(Self::HEIGHT - 1, Self::WIDTH - 1);
			return;
		}

		self.regularize();
	}

	/// if possible, move cursor absolutely.
	pub fn move_abs(&mut self, y: isize, x: isize) {
		self.do_move(y.clamp(0, Self::HEIGHT - 1), x.clamp(0, Self::WIDTH - 1));
	}

	/// move cursor absolutely but only x.
	pub fn move_abs_x(&mut self, x: isize) {
		self.move_abs(self.y, x)
	}

	/// move cursor absolutely but only y.
	pub fn move_abs_y(&mut self, y: isize) {
		self.move_abs(y, self.x)
	}

	/// check relative move is possible.
	pub fn check_rel(&mut self, dy: isize, dx: isize) -> Result<()> {
		match Self::is_regular(self.y + dy, self.x + dx) {
			true => Ok(()),
			false => Err(()),
		}
	}

	pub fn fixup_line_end(&mut self) {
		if self.x == Self::WIDTH - 1 {
			self.x -= 1;
		}
	}

	/// convert 2d coordinate into 1d offset.
	pub fn into_flat(self) -> usize {
		(self.y as usize) * WIDTH + (self.x as usize)
	}

	pub fn to_tuple(&self) -> (usize, usize) {
		(self.y as usize, self.x as usize)
	}
}

impl<const H: usize, const W: usize> From<Cursor<H, W>> for usize {
	fn from(value: Cursor<H, W>) -> Self {
		value.into_flat()
	}
}

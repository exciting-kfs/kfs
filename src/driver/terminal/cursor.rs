//! Console Cursor

use super::WinSize;

pub type Result<T> = core::result::Result<T, ()>;

#[derive(Clone, Copy)]
pub struct Cursor {
	bound: WinSize,
	y: isize,
	x: isize,
}

impl Cursor {
	/// construct new cursor pointing at (y, x)
	pub fn at(bound: WinSize, y: isize, x: isize) -> Result<Self> {
		let mut new = unsafe { Self::at_unchecked(bound, y, x) };

		if !new.is_valid(y, x) {
			return Err(());
		}

		new.regularize();

		Ok(new)
	}

	pub fn width(&self) -> isize {
		self.bound.col as isize
	}

	pub fn height(&self) -> isize {
		self.bound.row as isize
	}

	pub fn newline_width(&self) -> isize {
		self.width() - 1
	}

	pub fn new(bound: WinSize) -> Self {
		Cursor { bound, y: 0, x: 0 }
	}

	pub const unsafe fn at_unchecked(bound: WinSize, y: isize, x: isize) -> Self {
		Cursor { bound, y, x }
	}

	/// check `(y, x)` is `regular`.
	fn is_regular(&self, y: isize, x: isize) -> bool {
		let x_ok = 0 <= x && x < self.width();
		let y_ok = 0 <= y && y < self.height();

		x_ok && y_ok
	}

	/// check `(y, x)` is `valid`.
	fn is_valid(&self, y: isize, x: isize) -> bool {
		let flat = y * self.width() + x;

		0 <= flat && flat < self.width() * self.height()
	}

	fn do_move(&mut self, y: isize, x: isize) {
		self.x = x;
		self.y = y;
	}

	/// move cursor relatively.
	pub fn move_rel_y(&mut self, dy: isize) {
		self.y = (self.y + dy).clamp(0, self.height() - 1);
	}

	pub fn move_rel_x(&mut self, dx: isize) {
		self.x = (self.x + dx).clamp(0, self.width() - 1);
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
		self.y += self.x / self.width();
		self.x %= self.width();

		if self.x < 0 {
			self.y -= 1;
			self.x += self.width();
		}
	}

	pub fn move_rel_wrap_x(&mut self, dx: isize) {
		self.x += dx;

		let flat = self.y * self.width() + self.x;
		if flat < 0 {
			self.do_move(0, 0);
			return;
		}

		if flat >= self.width() * self.height() {
			self.do_move(self.height() - 1, self.width() - 1);
			return;
		}

		self.regularize();
	}

	/// if possible, move cursor absolutely.
	pub fn move_abs(&mut self, y: isize, x: isize) {
		self.do_move(y.clamp(0, self.height() - 1), x.clamp(0, self.width() - 1));
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
		match self.is_regular(self.y + dy, self.x + dx) {
			true => Ok(()),
			false => Err(()),
		}
	}

	pub fn fixup_line_end(&mut self) {
		if self.x == self.width() - 1 {
			self.x -= 1;
		}
	}

	/// convert 2d coordinate into 1d offset.
	pub fn into_flat(self) -> usize {
		(self.y as usize) * self.width() as usize + (self.x as usize)
	}

	pub fn to_tuple(&self) -> (usize, usize) {
		(self.y as usize, self.x as usize)
	}
}

impl From<Cursor> for usize {
	fn from(value: Cursor) -> Self {
		value.into_flat()
	}
}

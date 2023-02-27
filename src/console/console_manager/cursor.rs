//! Console Cursor

#[derive(Debug)]
pub enum Direction {
	Top,
	Bottom,
	Left,
	Right,
	TopLeft,
	TopRight,
	BottomLeft,
	BottomRight,
}

pub type Result<T> = core::result::Result<T, Direction>;

#[derive(Clone)]
pub struct Cursor<const H: usize, const W: usize> {
	y: isize,
	x: isize,
}

impl<const HEIGHT: usize, const WIDTH: usize> Cursor<HEIGHT, WIDTH> {
	/// construct new cursor pointing at (y, x)
	pub fn at(y: isize, x: isize) -> Result<Self> {
		Self::bound_check(y, x)?;
		Ok(Self { y, x })
	}

	pub const unsafe fn at_unchecked(y: isize, x: isize) -> Self {
		Cursor { y, x }
	}

	pub const fn new() -> Self {
		Cursor { y: 0, x: 0 }
	}

	/// check `(y, x)` is out of bound.
	///
	/// # Returns
	/// 	- `Err(e)`: that point is out of bound. `e` is OOB direction.
	///     - `Ok((y, x))`: that point is in-bound.
	fn bound_check(y: isize, x: isize) -> Result<(isize, isize)> {
		let overflow_top = 0 > y;
		let overflow_bottom = y >= HEIGHT as isize;
		let overflow_left = 0 > x;
		let overflow_right = x >= WIDTH as isize;

		match (overflow_top, overflow_bottom, overflow_left, overflow_right) {
			(true, false, false, false) => Err(Direction::Top),
			(false, true, false, false) => Err(Direction::Bottom),
			(false, false, true, false) => Err(Direction::Left),
			(false, false, false, true) => Err(Direction::Right),
			(true, false, true, false) => Err(Direction::TopLeft),
			(true, false, false, true) => Err(Direction::TopRight),
			(false, true, true, false) => Err(Direction::BottomLeft),
			(false, true, false, true) => Err(Direction::BottomRight),
			(false, false, false, false) => Ok((y, x)),
			_ => unreachable!("WIDTH or HEIGHT is zero"),
		}
	}

	fn do_move(&mut self, y: isize, x: isize) {
		self.x = x;
		self.y = y;
	}

	/// if possible, move cursor relatively.
	pub fn move_rel(&mut self, dy: isize, dx: isize) -> Result<()> {
		let (y, x) = Self::bound_check(self.y + dy, self.x + dx)?;

		self.do_move(y, x);

		Ok(())
	}

	/// move cursor relatively. if new point is OOB, then move partially.
	pub fn move_rel_partial(&mut self, dy: isize, dx: isize) {
		self.do_move(
			(self.y + dy).clamp(0, HEIGHT as isize - 1),
			(self.x + dx).clamp(0, WIDTH as isize - 1),
		);
	}

	/// check relative move is possible.
	pub fn check_rel(&mut self, dy: isize, dx: isize) -> Result<()> {
		Self::bound_check(self.y + dy, self.x + dx).map(|_| ())
	}

	/// if possible, move cursor absolutely.
	pub fn move_abs(&mut self, y: isize, x: isize) -> Result<()> {
		let (y, x) = Self::bound_check(y, x)?;

		self.do_move(y, x);

		Ok(())
	}

	/// move cursor absolutely but only x.
	pub fn move_abs_x(&mut self, x: isize) -> Result<()> {
		self.move_abs(self.y, x)
	}

	/// move cursor absolutely but only y.
	pub fn move_abs_y(&mut self, y: isize) -> Result<()> {
		self.move_abs(y, self.x)
	}

	/// convert 2d coordinate into 1d offset.
	pub fn to_idx(&self) -> usize {
		(self.y as usize) * WIDTH + (self.x as usize)
	}
}

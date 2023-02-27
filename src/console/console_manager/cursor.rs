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

#[derive(Clone, Copy)]
pub struct Cursor<const H: usize, const W: usize> {
	y: isize,
	x: isize,
}

impl<const HEIGHT: usize, const WIDTH: usize> Cursor<HEIGHT, WIDTH> {
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

	fn bound_check(y: isize, x: isize) -> Result<(isize, isize)> {
		let top = 0 <= y;
		let bottom = y < HEIGHT as isize;
		let left = x <= 0;
		let right = x < WIDTH as isize;

		match (top, bottom, left, right) {
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

	pub fn move_rel(&mut self, dy: isize, dx: isize) -> Result<()> {
		let (y, x) = Self::bound_check(self.y + dy, self.x + dx)?;

		self.do_move(y, x);

		Ok(())
	}

	pub fn move_rel_partial(&mut self, dy: isize, dx: isize) {
		self.do_move(
			(self.y + dy).clamp(0, HEIGHT as isize - 1),
			(self.x + dx).clamp(0, WIDTH as isize - 1),
		);
	}

	pub fn check_rel(&mut self, dy: isize, dx: isize) -> Result<()> {
		Self::bound_check(self.y + dy, self.x + dx).map(|_| ())
	}

	pub fn move_abs(&mut self, y: isize, x: isize) -> Result<()> {
		let (y, x) = Self::bound_check(y, x)?;

		self.do_move(y, x);

		Ok(())
	}

	pub fn move_abs_x(&mut self, x: isize) -> Result<()> {
		self.move_abs(self.y, x)
	}

	pub fn move_abs_y(&mut self, y: isize) -> Result<()> {
		self.move_abs(y, self.x)
	}

	pub fn to_idx(&self) -> usize {
		(self.y as usize) * WIDTH + (self.x as usize)
	}
}

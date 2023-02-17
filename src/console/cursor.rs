use crate::driver::vga::text_vga;
pub enum MoveResult {
	Pass,
	AdjustTop(isize),
}

#[derive(Clone, Copy)]
pub struct Cursor {
	pub y: usize,
	pub x: usize,
}

impl Cursor {
	pub const fn new(y: usize, x: usize) -> Self {
		Cursor { y, x }
	}

	pub fn relative_move(&mut self, dy: isize, dx: isize) -> MoveResult {
		let mut ret = MoveResult::Pass;

		let mut y = self.y as isize + dy;
		let mut x = self.x as isize + dx;

		let vga_width: isize = text_vga::WIDTH as isize;
		let vga_height: isize = text_vga::HEIGHT as isize;

		if x >= vga_width {
			y += 1;
			x = 0;
		} else if x < 0 {
			y -= 1;
			x = vga_width - 1;
		}

		if y < 0 {
			ret = MoveResult::AdjustTop(y);
			y = 0;
		} else if y >= vga_height {
			ret = MoveResult::AdjustTop(y - vga_height + 1);
			y = vga_height - 1;
		}

		self.x = x as usize;
		self.y = y as usize;

		ret
	}

	pub fn to_idx(&self) -> usize {
		self.y * text_vga::WIDTH + self.x
	}
}

use super::Color;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Attr(pub u8);

const FG_OFFSET: u8 = 0;
const BG_OFFSET: u8 = 4;

const FG_MASK: u8 = 0b1111 << FG_OFFSET;
const BG_MASK: u8 = 0b1111 << BG_OFFSET;

const DEFAULT_FG: Color = Color::White;
const DEFAULT_BG: Color = Color::Black;

impl Attr {
	pub const fn new(bg: Color, fg: Color) -> Self {
		Attr(((bg as u8) << BG_OFFSET) | ((fg as u8) << FG_OFFSET))
	}

	pub const fn default() -> Self {
		Self::new(DEFAULT_BG, DEFAULT_FG)
	}

	pub fn set_fg(&mut self, fg: Color) {
		self.0 = (self.0 & !FG_MASK) | ((fg as u8) << FG_OFFSET);
	}

	pub fn get_fg(&self) -> u8 {
		(self.0 & FG_MASK) >> FG_OFFSET
	}

	pub fn get_bg(&self) -> u8 {
		(self.0 & BG_MASK) >> BG_OFFSET
	}

	pub fn set_bg(&mut self, bg: Color) {
		self.0 = (self.0 & !BG_MASK) | ((bg as u8) << BG_OFFSET);
	}

	pub fn reset_fg(&mut self) {
		self.set_fg(DEFAULT_FG);
	}

	pub fn reset_bg(&mut self) {
		self.set_bg(DEFAULT_BG);
	}
}

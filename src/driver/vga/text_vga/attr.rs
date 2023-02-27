use super::Color;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Attr(pub u8);

const FG_OFFSET: u8 = 0;
const BG_OFFSET: u8 = 4;
const BLINK_OFFSET: u8 = 7;
const BRIGHT_OFFSET: u8 = 3;

const FG_MASK: u8 = 0b111 << FG_OFFSET;
const BG_MASK: u8 = 0b111 << BG_OFFSET;
const BLINK_MASK: u8 = 1 << BLINK_OFFSET;
const BRIGHT_MASK: u8 = 1 << BRIGHT_OFFSET;

const DEFAULT_FG: Color = Color::White;
const DEFAULT_BG: Color = Color::Black;

impl Attr {
	pub const fn new(blink: bool, bg: Color, bright: bool, fg: Color) -> Self {
		Attr(
			((blink as u8) << BLINK_OFFSET)
				| ((bg as u8) << BG_OFFSET)
				| ((bright as u8) << BRIGHT_OFFSET)
				| ((fg as u8) << FG_OFFSET),
		)
	}

	pub const fn default() -> Self {
		Self::new(false, DEFAULT_BG, false, DEFAULT_FG)
	}

	pub fn toggle_blink(&mut self) {
		self.0 ^= BLINK_MASK;
	}

	pub fn toggle_bright(&mut self) {
		self.0 ^= 1 << BLINK_MASK;
	}

	pub fn set_fg(&mut self, fg: Color) {
		self.0 = (self.0 & !FG_MASK) | ((fg as u8) << FG_OFFSET);
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

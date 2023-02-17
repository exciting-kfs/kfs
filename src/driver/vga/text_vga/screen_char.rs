use super::Attr;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Char(pub u16);

impl Char {
	pub const fn styled(attr: Attr, ch: u8) -> Self {
		Char(((attr.0 as u16) << 8) | (ch as u16))
	}

	pub const fn new(ch: u8) -> Self {
		Self::styled(Attr::default(), ch)
	}

	pub fn empty() -> Self {
		Self::styled(Attr::default(), b'\0')
	}
}

impl Default for Char {
	fn default() -> Self {
		Self::empty()
	}
}

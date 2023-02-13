use crate::input::key_event::Code;

pub struct KeyRecord {
	pub control: bool,
	pub alt: bool,
	pub printable: Code,
}

impl KeyRecord {
	pub fn new() -> Self {
		Self::default()
	}
}

impl Default for KeyRecord {
	fn default() -> Self {
		KeyRecord {
			control: false,
			alt: false,
			printable: Code::None,
		}
	}
}

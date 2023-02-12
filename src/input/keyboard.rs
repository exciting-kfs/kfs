use super::key_event::{Code, Key, KeyState, PrintVar};
use crate::driver::{ps2::keyboard::get_key_event, vga::text_vga};

#[derive(Default)]
pub struct Keyboard {
	shift: bool,
	control: bool,
	alt: bool,
	gui: bool,
	caps_lock: bool,
	num_lock: bool,
	scroll_lock: bool,
}

//TODO - unicode support?
pub struct KeyboardEvent {
	pub state: KeyState,
	pub key: Key,
	pub ascii: u8,
}

impl Keyboard {
	pub fn new() -> Self {
		Self::default()
	}

	fn handle_modifier_key(&mut self, code: Code, state: KeyState) {
		match code {
			Code::Shift => self.shift = state.into(),
			Code::Alt => self.alt = state.into(),
			Code::Gui => self.gui = state.into(),
			Code::Control => self.control = state.into(),
			_ => unreachable!("code is not for modifier key"),
		}
	}

	fn handle_toggle_key(&mut self, code: Code, state: KeyState) {
		if state != KeyState::Pressed {
			return;
		}

		match code {
			Code::Capslock => self.caps_lock = !self.caps_lock,
			Code::Numberlock => self.num_lock = !self.num_lock,
			Code::Scrolllock => self.scroll_lock = !self.scroll_lock,
			_ => unreachable!("code is not for toggle key"),
		}
	}

	fn alpha_to_ascii(&self, code: u8) -> u8 {
		let upper_case = self.caps_lock || self.shift;

		if upper_case {
			text_vga::putc(1, 1, text_vga::Char::new(b'Y'));
			code.to_ascii_uppercase()
		} else {
			text_vga::putc(1, 1, text_vga::Char::new(b'N'));
			code
		}
	}

	fn numpad_to_ascii(&self, code: u8, var: PrintVar) -> u8 {
		// convert to special char when
		//  (1). shift is pressed
		//  (2). key was not pressed from numpad

		let is_special = self.shift && var == PrintVar::Regular;

		if !is_special {
			return code;
		}

		match code {
			b'1' => b'!',
			b'2' => b'@',
			b'3' => b'#',
			b'4' => b'$',
			b'5' => b'%',
			b'6' => b'^',
			b'7' => b'&',
			b'8' => b'*',
			b'9' => b'(',
			b'0' => b')',
			b'-' => b'_',
			b'/' => b'?',
			b'.' => b'>',
			_ => unreachable!("code must be exist on both regular / numpad"),
		}
	}

	fn others_to_ascii(&self, code: u8) -> u8 {
		let alternate = self.shift;

		if !alternate {
			return code;
		}

		match code {
			b'`' => b'~',
			b'=' => b'+',
			b'[' => b'{',
			b']' => b'}',
			b'\\' => b'|',
			b';' => b':',
			b'\'' => b'"',
			b',' => b'>',
			_ => unreachable!("unknown code for others"),
		}
	}

	fn printable_to_ascii(&self, code: u8, var: PrintVar) -> u8 {
		match code {
			b'a'..=b'z' => self.alpha_to_ascii(code),
			b'0'..=b'9' | b'-' | b'/' | b'.' => self.numpad_to_ascii(code, var),
			b'`' | b'=' | b'[' | b']' | b'\\' | b';' | b'\'' | b',' => self.others_to_ascii(code),
			_ => unreachable!("unknown code detected"),
		}
	}

	pub fn get_keyboard_event(&mut self) -> Option<KeyboardEvent> {
		let event = match get_key_event() {
			Some(ev) => ev,
			None => return None,
		};

		match event.key {
			Key::Modifier(key, ..) => self.handle_modifier_key(key, event.state),
			Key::Toggle(key) => self.handle_toggle_key(key, event.state),
			_ => (),
		};

		let ascii = match event.key {
			Key::Printable(key, var) => self.printable_to_ascii(key as u8, var),
			_ => b'\0',
		};

		Some(KeyboardEvent {
			state: event.state,
			key: event.key,
			ascii,
		})
	}

	pub fn wait_keyboard_event(&mut self) -> KeyboardEvent {
		loop {
			if let Some(ev) = self.get_keyboard_event() {
				return ev;
			}
		}
	}
}

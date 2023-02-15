//! Implements common keyboard tasks
//!
//! ### some example of common tasks
//!  - save modifier / toggle keys state
//!  - convert pressed key to ascii representation (if possible)
//!  - key repeat rate / threshold

use super::key_event::{Code, Key, KeyState, PrintVar};
use crate::driver::ps2::keyboard::get_key_event;

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

/// General keyboard event
///
/// - `state`: either key is pressed or released.
/// - `key`: **exact** related key.
/// - `ascii`: ascii representation of key.
pub struct KeyboardEvent {
	pub state: KeyState,
	pub key: Key,
	pub ascii: u8,
}

impl Keyboard {
	pub fn new() -> Self {
		Self::default() // false, false, false ...
	}

	/// 키보드에서 키 하나를 입력받고, 상태를 저장한 후, 받은 키를 반환한다.
	///
	/// # Returns
	///  - `None` -> 현재 키보드 버퍼에서 읽을 키가 존재하지 않음.
	///  - `Some(x)` -> 읽은 키에 대한 정보
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

	/// wait until key is pressed, then return received event.
	pub fn wait_keyboard_event(&mut self) -> KeyboardEvent {
		loop {
			if let Some(ev) = self.get_keyboard_event() {
				return ev;
			}
		}
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

	/// 알파벳 대소문자 처리
	fn alpha_to_ascii(&self, code: u8) -> u8 {
		let upper_case = self.caps_lock || self.shift;

		if upper_case {
			code.to_ascii_uppercase()
		} else {
			code
		}
	}

	/// 표준 배열과 넘패드에 동시에 존재하는 키를 ascii로 변환함.
	///
	/// 만약 시프트가 눌렸고, 눌린 키가 넘패드에서 눌린 것이 아닌 경우
	/// 추가적인 변환이 일어남.
	fn numpad_to_ascii(&self, code: u8, var: PrintVar) -> u8 {
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

	/// 추가적으로 shift를 눌렀을 때 변화가 일어나야 하는 키들의 처리
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

	/// shift / capslock 등 현재 키 입력 상태에 따라 다른 ascii 표현을 가지는 키를 처리
	fn printable_to_ascii(&self, code: u8, var: PrintVar) -> u8 {
		match code {
			b'a'..=b'z' => self.alpha_to_ascii(code),
			b'0'..=b'9' | b'-' | b'/' | b'.' => self.numpad_to_ascii(code, var),
			b'`' | b'=' | b'[' | b']' | b'\\' | b';' | b'\'' | b',' => self.others_to_ascii(code),
			_ => unreachable!("unknown code detected"),
		}
	}
}

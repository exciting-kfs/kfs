use super::control::{test_status_now, Status};
use crate::input::key_event::{Code, KeyEvent, KeyState};
use crate::io::pmio::Port;
use crate::sync::locked::Locked;

pub(super) static KEYBOARD_PORT: Port = Port::new(0x60);

const CODE_PAGE2: u8 = 0xe0;
const PAUSE: u8 = 0xe1;

const PRINT_SCREEN_PRESS: u8 = 0x2a;
const PRINT_SCREEN_RELEASE: u8 = 0xb7;

pub fn available() -> bool {
	test_status_now(Status::OBF)
}

pub fn poll_raw_scancode() -> Option<u8> {
	if available() {
		Some(get_raw_scancode())
	} else {
		None
	}
}

pub fn get_raw_scancode() -> u8 {
	KEYBOARD_PORT.read_byte()
}

pub fn wait_raw_scancode() -> u8 {
	loop {
		match poll_raw_scancode() {
			Some(c) => return c,
			None => continue,
		}
	}
}

fn ignore_scancodes(seq: &[u8]) {
	for byte in seq {
		let code = poll_raw_scancode().expect("buffer excedeed before end of scancodes.");

		if *byte != code {
			panic!("scancode mismatch. expected={byte}, got={code}");
		}
	}
}

fn get_pause_keyevent() -> KeyEvent {
	ignore_scancodes(&[0x1D, 0x45, 0xE1, 0x9D, 0xC5]);

	KeyEvent {
		state: KeyState::Pressed,
		key: Code::Pause,
	}
}

fn scancode_to_keyevent(page: usize, code: u8) -> KeyEvent {
	let state = match (code & 128) != 0 {
		true => KeyState::Released,
		false => KeyState::Pressed,
	};

	let key = SCAN_CODE_SET1[page][(code & !128) as usize];

	KeyEvent { state, key }
}

pub fn wait_key_event() -> KeyEvent {
	loop {
		match get_key_event() {
			Some(ev) => return ev,
			None => continue,
		}
	}
}

/// Get current key event.
///
/// if there is no available event, then returns None.
pub fn get_key_event() -> Option<KeyEvent> {
	let mut raw_scancode = poll_raw_scancode()?;
	let page = match raw_scancode {
		PAUSE => return Some(get_pause_keyevent()),
		CODE_PAGE2 => {
			raw_scancode = poll_raw_scancode().expect("buffer excedeed before end of scancodes.");
			1
		}
		_ => 0,
	};
	Some(scancode_to_keyevent(page, raw_scancode))
}

/// Change raw_scancode into KeyEvent
pub fn into_key_event(raw_scancode: u8) -> Option<KeyEvent> {
	let mut kb = PS2_KEYBOARD.lock();
	kb.update(raw_scancode);
	kb.get_key_event()
}

struct Keyboard {
	state: State,
	code: u8,
	count: u8,
}

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
enum State {
	Page0,
	Page1,
	Pause,
}

impl Keyboard {
	fn update(&mut self, raw_scancode: u8) {
		match raw_scancode {
			PAUSE => self.state = State::Pause,
			CODE_PAGE2 => self.state = State::Page1,
			_ => {}
		}

		self.code = raw_scancode;
		self.count += 1;
	}

	fn get_key_event(&mut self) -> Option<KeyEvent> {
		let state = self.state;
		let count = self.count;
		let code = self.code;

		match (state, count) {
			(State::Page0, 1) => Some(scancode_to_keyevent(0, code)),
			(State::Page1, 2) => Some(scancode_to_keyevent(1, code)),
			(State::Pause, 6) => Some(KeyEvent {
				state: KeyState::Pressed,
				key: Code::Pause,
			}),
			_ => None,
		}
		.map(|ev| {
			self.reset();
			ev
		})
	}

	fn reset(&mut self) {
		self.state = State::Page0;
		self.count = 0;
	}
}

static PS2_KEYBOARD: Locked<Keyboard> = Locked::new(Keyboard {
	state: State::Page0,
	code: 0,
	count: 0,
});

/// PS/2 SCAN CODE SET 1 to `Key` translate table.
///
/// - SCAN_CODE_SET1\[0\] -> table for **not** starting with 0xe0
/// - SCAN_CODE_SET1\[1\] -> table for starting with 0xe0
///
/// **Generated by script. do not touch manually.**
static SCAN_CODE_SET1: [[Code; 128]; 2] = [
	[
		Code::Unknown,
		Code::Escape,
		Code::N1,
		Code::N2,
		Code::N3,
		Code::N4,
		Code::N5,
		Code::N6,
		Code::N7,
		Code::N8,
		Code::N9,
		Code::N0,
		Code::Minus,
		Code::Equal,
		Code::Backspace,
		Code::Tab,
		Code::Q,
		Code::W,
		Code::E,
		Code::R,
		Code::T,
		Code::Y,
		Code::U,
		Code::I,
		Code::O,
		Code::P,
		Code::BracketOpen,
		Code::BracketClose,
		Code::Enter,
		Code::LControl,
		Code::A,
		Code::S,
		Code::D,
		Code::F,
		Code::G,
		Code::H,
		Code::J,
		Code::K,
		Code::L,
		Code::Semicolon,
		Code::Quote,
		Code::Backtick,
		Code::LShift,
		Code::Backslash,
		Code::Z,
		Code::X,
		Code::C,
		Code::V,
		Code::B,
		Code::N,
		Code::M,
		Code::Comma,
		Code::Dot,
		Code::Slash,
		Code::RShift,
		Code::KpMultiply,
		Code::LAlt,
		Code::Space,
		Code::Capslock,
		Code::F1,
		Code::F2,
		Code::F3,
		Code::F4,
		Code::F5,
		Code::F6,
		Code::F7,
		Code::F8,
		Code::F9,
		Code::F10,
		Code::Numlock,
		Code::ScrollLock,
		Code::KpN7,
		Code::KpN8,
		Code::KpN9,
		Code::KpMinus,
		Code::KpN4,
		Code::KpN5,
		Code::KpN6,
		Code::KpPlus,
		Code::KpN1,
		Code::KpN2,
		Code::KpN3,
		Code::KpN0,
		Code::KpDot,
		Code::PrintScreen,
		Code::Unknown,
		Code::Unknown,
		Code::F11,
		Code::F12,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
	],
	[
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::KpEnter,
		Code::RControl,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::KpSlash,
		Code::Unknown,
		Code::Unknown,
		Code::RAlt,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Home,
		Code::Up,
		Code::PageUp,
		Code::Unknown,
		Code::Left,
		Code::Unknown,
		Code::Right,
		Code::Unknown,
		Code::End,
		Code::Down,
		Code::PageDown,
		Code::Insert,
		Code::Delete,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::LGui,
		Code::RGui,
		Code::Apps,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
		Code::Unknown,
	],
];

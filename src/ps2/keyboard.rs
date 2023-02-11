use super::control::{test_status_now, Status};
use crate::pmio::Port;

static KEYBOARD_PORT: Port = Port::new(0x60);

const CODE_PAGE2: u8 = 0xe0;
const PAUSE: u8 = 0xe1;

const PRINT_SCREEN_PRESS: u8 = 0x2a;
const PRINT_SCREEN_RELEASE: u8 = 0xb7;

pub fn available() -> bool {
	test_status_now(Status::OBF)
}

pub fn get_raw_scancode() -> Option<u8> {
	if available() {
		Some(KEYBOARD_PORT.read_byte())
	} else {
		None
	}
}

pub fn wait_raw_scancode() -> u8 {
	loop {
		match get_raw_scancode() {
			Some(c) => return c,
			None => continue,
		}
	}
}

fn ignore_scancodes(seq: &[u8]) {
	for byte in seq {
		let code = get_raw_scancode().expect("buffer excedeed before end of scancodes.");

		if *byte != code {
			panic!("scancode mismatch. expected={byte}, got={code}");
		}
	}
}

fn get_pause_keyevent() -> KeyEvent {
	ignore_scancodes(&[0x1D, 0x45, 0xE1, 0x9D, 0xC5]);

	KeyEvent {
		state: KeyState::Pressed,
		kind: KeyKind::Control,
		key: Key::Pause,
	}
}

fn get_print_screen_press_keyevent() -> KeyEvent {
	ignore_scancodes(&[0xE0, 0x37]);

	KeyEvent {
		state: KeyState::Pressed,
		kind: KeyKind::Control,
		key: Key::PrintScreen,
	}
}

fn get_print_screen_release_keyevent() -> KeyEvent {
	ignore_scancodes(&[0xE0, 0xAA]);

	KeyEvent {
		state: KeyState::Released,
		kind: KeyKind::Control,
		key: Key::PrintScreen,
	}
}

fn scancode_to_keyevent(page: usize, code: u8) -> KeyEvent {
	let state = match (code & 128) != 0 {
		true => KeyState::Released,
		false => KeyState::Pressed,
	};

	let key = SCAN_CODE_SET1[page][(code & !128) as usize];

	KeyEvent {
		state,
		kind: KeyKind::Printable,
		key,
	}
}

pub fn wait_key_event() -> KeyEvent {
	loop {
		match get_key_event() {
			Some(ev) => return ev,
			None => continue,
		}
	}
}

pub fn get_key_event() -> Option<KeyEvent> {
	let raw_scancode = get_raw_scancode();

	let mut raw_scancode = match raw_scancode {
		Some(v) => v,
		None => return None,
	};

	let page = match raw_scancode {
		PAUSE => return Some(get_pause_keyevent()),
		CODE_PAGE2 => {
			raw_scancode = get_raw_scancode().expect("buffer excedeed before end of scancodes.");

			match raw_scancode {
				PRINT_SCREEN_PRESS => return Some(get_print_screen_press_keyevent()),
				PRINT_SCREEN_RELEASE => return Some(get_print_screen_release_keyevent()),
				_ => 1,
			}
		}
		_ => 0,
	};

	Some(scancode_to_keyevent(page, raw_scancode))
}

static SCAN_CODE_SET1: [[Key; 128]; 2] = [
	[
		Key::Unused,
		Key::Escape,
		Key::N1,
		Key::N2,
		Key::N3,
		Key::N4,
		Key::N5,
		Key::N6,
		Key::N7,
		Key::N8,
		Key::N9,
		Key::N0,
		Key::Minus,
		Key::Equal,
		Key::Backspace,
		Key::Tab,
		Key::Q,
		Key::W,
		Key::E,
		Key::R,
		Key::T,
		Key::Y,
		Key::U,
		Key::I,
		Key::O,
		Key::P,
		Key::BracketOpen,
		Key::BracketClose,
		Key::Enter,
		Key::LeftControl,
		Key::A,
		Key::S,
		Key::D,
		Key::F,
		Key::G,
		Key::H,
		Key::J,
		Key::K,
		Key::L,
		Key::Semicolon,
		Key::SingleQuote,
		Key::Backtick,
		Key::LeftShift,
		Key::Backslash,
		Key::Z,
		Key::X,
		Key::C,
		Key::V,
		Key::B,
		Key::N,
		Key::M,
		Key::Comma,
		Key::Dot,
		Key::Slash,
		Key::RightShift,
		Key::KeypadAsterisk,
		Key::LeftAlt,
		Key::Space,
		Key::Capslock,
		Key::F1,
		Key::F2,
		Key::F3,
		Key::F4,
		Key::F5,
		Key::F6,
		Key::F7,
		Key::F8,
		Key::F9,
		Key::F10,
		Key::Numberlock,
		Key::Scrolllock,
		Key::KeypadN7,
		Key::KeypadN8,
		Key::KeypadN9,
		Key::KeypadMinus,
		Key::KeypadN4,
		Key::KeypadN5,
		Key::KeypadN6,
		Key::KeypadPlus,
		Key::KeypadN1,
		Key::KeypadN2,
		Key::KeypadN3,
		Key::KeypadN0,
		Key::KeypadDot,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::F11,
		Key::F12,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
	],
	[
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::MultimediaPreviousTrack,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::MultimediaNextTrack,
		Key::Unused,
		Key::Unused,
		Key::KeypadEnter,
		Key::RightControl,
		Key::Unused,
		Key::Unused,
		Key::MultimediaMute,
		Key::MultimediaCalculator,
		Key::MultimediaPlay,
		Key::Unused,
		Key::MultimediaStop,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::MultimediaVolumeDown,
		Key::Unused,
		Key::MultimediaVolumeUp,
		Key::Unused,
		Key::MultimediaWwwHome,
		Key::Unused,
		Key::Unused,
		Key::KeypadSlash,
		Key::Unused,
		Key::Unused,
		Key::RightAlt,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Home,
		Key::ArrowUp,
		Key::PageUp,
		Key::Unused,
		Key::ArrowLeft,
		Key::Unused,
		Key::ArrowRight,
		Key::Unused,
		Key::End,
		Key::ArrowDown,
		Key::PageDown,
		Key::Insert,
		Key::Delete,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::LeftGui,
		Key::RightGui,
		Key::Apps,
		Key::AcpiPower,
		Key::AcpiSleep,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::AcpiWake,
		Key::Unused,
		Key::MultimediaWwwSearch,
		Key::MultimediaWwwFavorites,
		Key::MultimediaWwwRefresh,
		Key::MultimediaWwwStop,
		Key::MultimediaWwwForward,
		Key::MultimediaWwwBack,
		Key::MultimediaMyComputer,
		Key::MultimediaEmail,
		Key::MultimediaMediaSelect,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
		Key::Unused,
	],
];

pub enum KeyKind {
	Printable,
	Control,
	Media,
	Acpi,
	Arrow,
}

#[rustfmt::skip]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Key {
	Unused,
	Escape,   F1, F2, F3, F4,   F5, F6, F7, F8,   F9, F10, F11, F12,
	Backtick, N1, N2, N3, N4, N5, N6, N7, N8, N9, N0, Minus, Equal, Backspace,
	Tab,       Q, W, E, R, T, Y, U, I, O, P, BracketOpen, BracketClose, Backslash,
	Capslock,   A, S, D, F, G, H, J, K, L, Semicolon, SingleQuote, Enter,
	LeftShift,   Z, X, C, V, B, N, M, Comma, Dot, Slash, RightShift,
	LeftControl, LeftGui, LeftAlt, Space, RightAlt, RightGui, RightControl,

	PrintScreen,  Pause, Scrolllock,
	Insert,       Home,  PageUp,
	Delete,       End,   PageDown,

	ArrowLeft, ArrowDown, ArrowUp, ArrowRight,

	Numberlock,  KeypadSlash, KeypadAsterisk,
	KeypadMinus, KeypadPlus,  KeypadEnter,
	KeypadN7,    KeypadN8,    KeypadN9,
	KeypadN4,    KeypadN5,    KeypadN6,
	KeypadN1,    KeypadN2,    KeypadN3,
	KeypadN0,    KeypadDot,

	MultimediaPreviousTrack, MultimediaNextTrack,    MultimediaMute,
	MultimediaCalculator,    MultimediaPlay,         MultimediaStop,
	MultimediaVolumeDown,    MultimediaVolumeUp,     MultimediaWwwHome,
	MultimediaWwwSearch,     MultimediaWwwFavorites, MultimediaWwwRefresh,
	MultimediaWwwStop,       MultimediaWwwForward,   MultimediaWwwBack,
	MultimediaMyComputer,    MultimediaEmail,        MultimediaMediaSelect,

	Apps, AcpiPower, AcpiSleep, AcpiWake,
}

pub enum KeyState {
	Pressed,
	Released,
}

pub struct KeyEvent {
	pub state: KeyState,
	pub kind: KeyKind,
	pub key: Key,
}

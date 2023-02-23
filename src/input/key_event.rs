//! Represent raw keyboard event

#![allow(dead_code)]

/// ANSI 104 key
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Code {
	Unknown,

	// Printable
	N1,
	N2,
	N3,
	N4,
	N5,
	N6,
	N7,
	N8,
	N9,
	N0,
	Minus,
	Equal,
	Q,
	W,
	E,
	R,
	T,
	Y,
	U,
	I,
	O,
	P,
	BracketOpen,
	BracketClose,
	A,
	S,
	D,
	F,
	G,
	H,
	J,
	K,
	L,
	Semicolon,
	Quote,
	Backtick,
	Backslash,
	Z,
	X,
	C,
	V,
	B,
	N,
	M,
	Comma,
	Dot,
	Slash,
	Space,

	// Modifier
	LControl,
	RControl,
	LShift,
	RShift,
	LAlt,
	RAlt,
	LGui,
	RGui,

	// Toggle
	Capslock,
	Numlock,
	ScrollLock,

	// Function
	F1,
	F2,
	F3,
	F4,
	F5,
	F6,
	F7,
	F8,
	F9,
	F10,
	F11,
	F12,

	// Keypad
	KpN1,
	KpN2,
	KpN3,
	KpN4,
	KpN5,
	KpN6,
	KpN7,
	KpN8,
	KpN9,
	KpN0,
	KpMinus,
	KpPlus,
	KpDot,
	KpSlash,
	KpMultiply,
	KpEnter,

	// Cursor
	Up,
	Down,
	Left,
	Right,
	PageUp,
	PageDown,
	Home,
	End,

	// Control
	Backspace,
	Delete,
	Tab,
	Enter,
	Insert,
	Pause,
	Escape,
	PrintScreen,
	Apps,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PrintableCode {
	N1 = Code::N1 as u8,
	N2 = Code::N2 as u8,
	N3 = Code::N3 as u8,
	N4 = Code::N4 as u8,
	N5 = Code::N5 as u8,
	N6 = Code::N6 as u8,
	N7 = Code::N7 as u8,
	N8 = Code::N8 as u8,
	N9 = Code::N9 as u8,
	N0 = Code::N0 as u8,
	Minus = Code::Minus as u8,
	Equal = Code::Equal as u8,
	Q = Code::Q as u8,
	W = Code::W as u8,
	E = Code::E as u8,
	R = Code::R as u8,
	T = Code::T as u8,
	Y = Code::Y as u8,
	U = Code::U as u8,
	I = Code::I as u8,
	O = Code::O as u8,
	P = Code::P as u8,
	BracketOpen = Code::BracketOpen as u8,
	BracketClose = Code::BracketClose as u8,
	A = Code::A as u8,
	S = Code::S as u8,
	D = Code::D as u8,
	F = Code::F as u8,
	G = Code::G as u8,
	H = Code::H as u8,
	J = Code::J as u8,
	K = Code::K as u8,
	L = Code::L as u8,
	Semicolon = Code::Semicolon as u8,
	Quote = Code::Quote as u8,
	Backtick = Code::Backtick as u8,
	Backslash = Code::Backslash as u8,
	Z = Code::Z as u8,
	X = Code::X as u8,
	C = Code::C as u8,
	V = Code::V as u8,
	B = Code::B as u8,
	N = Code::N as u8,
	M = Code::M as u8,
	Comma = Code::Comma as u8,
	Dot = Code::Dot as u8,
	Slash = Code::Slash as u8,
	Space = Code::Space as u8,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ModifierCode {
	LControl = Code::LControl as u8,
	RControl = Code::RControl as u8,
	LShift = Code::LShift as u8,
	RShift = Code::RShift as u8,
	LAlt = Code::LAlt as u8,
	RAlt = Code::RAlt as u8,
	LGui = Code::LGui as u8,
	RGui = Code::RGui as u8,
}

#[repr(u8)]
pub enum ToggleCode {
	Capslock = Code::Capslock as u8,
	Numlock = Code::Numlock as u8,
	ScrollLock = Code::ScrollLock as u8,
}

#[repr(u8)]
pub enum FunctionCode {
	F1 = Code::F1 as u8,
	F2 = Code::F2 as u8,
	F3 = Code::F3 as u8,
	F4 = Code::F4 as u8,
	F5 = Code::F5 as u8,
	F6 = Code::F6 as u8,
	F7 = Code::F7 as u8,
	F8 = Code::F8 as u8,
	F9 = Code::F9 as u8,
	F10 = Code::F10 as u8,
	F11 = Code::F11 as u8,
	F12 = Code::F12 as u8,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KeypadCode {
	KpN1 = Code::KpN1 as u8,
	KpN2 = Code::KpN2 as u8,
	KpN3 = Code::KpN3 as u8,
	KpN4 = Code::KpN4 as u8,
	KpN5 = Code::KpN5 as u8,
	KpN6 = Code::KpN6 as u8,
	KpN7 = Code::KpN7 as u8,
	KpN8 = Code::KpN8 as u8,
	KpN9 = Code::KpN9 as u8,
	KpN0 = Code::KpN0 as u8,
	KpMinus = Code::KpMinus as u8,
	KpPlus = Code::KpPlus as u8,
	KpDot = Code::KpDot as u8,
	KpSlash = Code::KpSlash as u8,
	KpMultiply = Code::KpMultiply as u8,
	KpEnter = Code::KpEnter as u8,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CursorCode {
	Up = Code::Up as u8,
	Down = Code::Down as u8,
	Left = Code::Left as u8,
	Right = Code::Right as u8,
	PageUp = Code::PageUp as u8,
	PageDown = Code::PageDown as u8,
	Home = Code::Home as u8,
	End = Code::End as u8,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ControlCode {
	Backspace = Code::Backspace as u8,
	Delete = Code::Delete as u8,
	Tab = Code::Tab as u8,
	Enter = Code::Enter as u8,
	Insert = Code::Insert as u8,
	Pause = Code::Pause as u8,
	Escape = Code::Escape as u8,
	PrintScreen = Code::PrintScreen as u8,
	Apps = Code::Apps as u8,
}

pub enum KeyKind {
	Printable(PrintableCode),
	Modifier(ModifierCode),
	Toggle(ToggleCode),
	Function(FunctionCode),
	Keypad(KeypadCode),
	Cursor(CursorCode),
	Control(ControlCode),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KeyState {
	Pressed,
	Released,
}

impl Into<bool> for KeyState {
	fn into(self) -> bool {
		match self {
			Self::Pressed => true,
			Self::Released => false,
		}
	}
}

impl From<bool> for KeyState {
	fn from(value: bool) -> Self {
		match value {
			true => Self::Pressed,
			false => Self::Released,
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub struct KeyEvent {
	pub state: KeyState,
	pub key: Code,
}

impl KeyEvent {
	pub fn identify(&self) -> KeyKind {
		if self.key == Code::Unknown {
			panic!("unknown key detected.");
		}

		// TODO: reduce if statments (binary branching or hash?)
		unsafe {
			use core::mem::transmute;
			if self.key <= Code::Space {
				return KeyKind::Printable(transmute(self.key));
			} else if self.key <= Code::RGui {
				return KeyKind::Modifier(transmute(self.key));
			} else if self.key <= Code::ScrollLock {
				return KeyKind::Toggle(transmute(self.key));
			} else if self.key <= Code::F12 {
				return KeyKind::Function(transmute(self.key));
			} else if self.key <= Code::KpEnter {
				return KeyKind::Keypad(transmute(self.key));
			} else if self.key <= Code::End {
				return KeyKind::Cursor(transmute(self.key));
			} else if self.key <= Code::Apps {
				return KeyKind::Control(transmute(self.key));
			} else {
				panic!("unknown key detected.");
			}
		}
	}

	pub fn pressed(&self) -> bool {
		self.state.into()
	}
}

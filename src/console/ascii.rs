use crate::input::key_event::*;
use crate::input::keyboard::Keyboard;

#[rustfmt::skip]
static ALPHA_LOWER: [u8; 26] = [
	b'a', b'b', b'c', b'd', b'e',
	b'f', b'g', b'h', b'i', b'j',
	b'k', b'l', b'm', b'n', b'o',
	b'p', b'q', b'r', b's', b't',
	b'u', b'v', b'w', b'x', b'y',
	b'z',
];

#[rustfmt::skip]
static ALPHA_UPPER: [u8; 26] = [
	b'A', b'B', b'C', b'D', b'E',
	b'F', b'G', b'H', b'I', b'J',
	b'K', b'L', b'M', b'N', b'O',
	b'P', b'Q', b'R', b'S', b'T',
	b'U', b'V', b'W', b'X', b'Y',
	b'Z',
];

#[rustfmt::skip]
static SYMBOL_PLAIN: [u8; 22] = [
	b'0',	b'1',	b'2',	b'3',	b'4',
	b'5',	b'6',	b'7',	b'8',	b'9',
	b'`',	b'-',	b'=',	b'[',	b']',
	b'\\',	b';',	b'\'',	b',',	b'.',
	b'/',	b' ',
];

#[rustfmt::skip]
static SYMBOL_SHIFT: [u8; 22] = [
	b')',	b'!',	b'@',	b'#',	b'$',
	b'%',	b'^',	b'&',	b'*',	b'(',
	b'~',	b'_',	b'+',	b'{',	b'}',
	b'|',	b':',	b'"',	b'<',	b'>',
	b'?',	b' ',
];

#[rustfmt::skip]
static FUNCTION: [&[u8]; 12] = [
	b"\x1bOP", b"\x1bOQ", b"\x1bOR", b"\x1bOS",
	b"\x1bOT", b"\x1bOU", b"\x1bOV", b"\x1bOW",
	b"\x1bOX", b"\x1bOY", b"\x1bOZ", b"\x1bO[",
];

// TODO: implement KEYPAD_NUMLOCK
#[rustfmt::skip]
static KEYPAD_PLAIN: [u8; 16] = [
	b'0', b'1', b'2', b'3',
	b'4', b'5', b'6', b'7',
	b'8', b'9', b'-', b'+',
	b'.', b'/', b'*', b'\n',
];

#[rustfmt::skip]
static CURSOR: [&[u8]; 8] = [
	b"\x1b[A",	b"\x1b[B",
	b"\x1b[D",	b"\x1b[C",
	b"\x1b[5~",	b"\x1b[6~",
	b"\x1b[H",	b"\x1b[F",
];

pub fn convert(code: Code, kbd: &Keyboard) -> Option<&'static [u8]> {
	match code.identify() {
		KeyKind::Alpha(code) => convert_alpha(code, kbd),
		KeyKind::Symbol(code) => convert_symbol(code, kbd),
		KeyKind::Function(code) => convert_function(code),
		KeyKind::Keypad(code) => convert_keypad(code),
		KeyKind::Cursor(code) => convert_cursor(code),
		KeyKind::Control(code) => convert_control(code),
		KeyKind::Modifier(code) => None,
		KeyKind::Toggle(code) => None,
	}
}

fn convert_alpha(code: AlphaCode, kbd: &Keyboard) -> Option<&'static [u8]> {
	let table = match kbd.shift_pressed() ^ kbd.pressed(Code::Capslock) {
		true => &ALPHA_UPPER,
		false => &ALPHA_LOWER,
	};

	let idx = code.index() as usize;
	Some(&table[idx..=idx])
}

fn convert_symbol(code: SymbolCode, kbd: &Keyboard) -> Option<&'static [u8]> {
	let table = match kbd.shift_pressed() {
		true => &SYMBOL_SHIFT,
		false => &SYMBOL_PLAIN,
	};

	let idx = code.index() as usize;
	Some(&table[idx..=idx])
}

fn convert_function(code: FunctionCode) -> Option<&'static [u8]> {
	Some(&FUNCTION[code.index() as usize])
}

fn convert_cursor(code: CursorCode) -> Option<&'static [u8]> {
	Some(&CURSOR[code.index() as usize])
}

fn convert_control(code: ControlCode) -> Option<&'static [u8]> {
	match code {
		ControlCode::Backspace => Some(b"\x7f"),
		ControlCode::Delete => Some(b"\x1b[3~"),
		ControlCode::Tab => Some(b"\t"),
		ControlCode::Enter => Some(b"\n"),
		ControlCode::Escape => Some(b"\x1b"),
		_ => None,
	}
}

fn convert_keypad(code: KeypadCode) -> Option<&'static [u8]> {
	let idx = code.index() as usize;
	Some(&KEYPAD_PLAIN[idx..=idx])
}

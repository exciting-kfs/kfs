//! Translate key code to ascii code
//! and do basic line discipline.

use alloc::collections::VecDeque;
use alloc::sync::{Arc, Weak};

use super::ascii::constants::*;
use super::console::Console;
use super::get_screen_draw_work;
use super::termios::{self, InputFlag, LocalFlag, OutputFlag, Termios, WinSize};

use crate::collection::LineBuffer;
use crate::driver::terminal::termios::{VINTR, VQUIT};
use crate::fs::vfs::{FileHandle, IOFlag};
use crate::input::key_event::*;
use crate::input::keyboard::KEYBOARD;
use crate::io::{BlkRead, BlkWrite, ChRead, ChWrite, NoSpace};
use crate::mm::user::verify::{verify_ptr, verify_ptr_mut};
use crate::pr_warn;
use crate::process::relation::session::Session;
use crate::process::relation::{Pgid, Pid};
use crate::process::signal::{poll_signal_queue, send_signal_to_foreground};
use crate::process::task::CURRENT;
use crate::process::wait_list::WaitList;
use crate::scheduler::preempt::{preempt_disable, AtomicOps};
use crate::scheduler::sleep::{sleep_and_yield_atomic, Sleep};
use crate::scheduler::work::{schedule_work, Work};
use crate::sync::{Locked, LockedGuard};
use crate::syscall::errno::Errno;

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
	b"\x1b[11~", b"\x1b[12~", b"\x1b[13~", b"\x1b[14~",
	b"\x1b[15~", b"\x1b[17~", b"\x1b[18~", b"\x1b[19~",
	b"\x1b[20~", b"\x1b[21~", b"\x1b[23~", b"\x1b[24~",
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

#[rustfmt::skip]
static CONTROL: [u8; 33] = [
	0x7f, 0x00, 0x01, 0x02,  ETX,  EOF, 0x05, 0x06, 0x07,
	  BS,   HT,   LF,  VT,    FF,   CR, 0x0e, 0x0f,
	0x10, 0x11, 0x12, 0x13, 0x14,  NAK, 0x16, 0x17,
	0x18, 0x19, 0x1a,  ESC,   FS, 0x1d, 0x1e, 0x1f,
];

fn convert_function(code: FunctionCode) -> Option<&'static [u8]> {
	Some(&FUNCTION[code.index() as usize])
}

fn convert_cursor(code: CursorCode) -> Option<&'static [u8]> {
	Some(&CURSOR[code.index() as usize])
}

fn convert_keypad(code: KeypadCode) -> Option<&'static [u8]> {
	let idx = code.index() as usize;
	Some(&KEYPAD_PLAIN[idx..=idx])
}

fn control_convertable(c: u8) -> bool {
	b'@' <= c && c <= b'_' || c == b'?'
}

fn convert_alpha(code: AlphaCode) -> Option<&'static [u8]> {
	let kbd = unsafe { &KEYBOARD };
	let idx = code.index() as usize;
	let ascii = ALPHA_UPPER[idx];

	if kbd.control_pressed() && control_convertable(ascii) {
		let idx = ascii.wrapping_sub(b'@').wrapping_add(1) as usize;
		Some(&CONTROL[idx..=idx])
	} else {
		let upper = kbd.shift_pressed() ^ kbd.pressed(Code::Capslock);
		let table = match upper {
			true => &ALPHA_UPPER,
			false => &ALPHA_LOWER,
		};

		Some(&table[idx..=idx])
	}
}

fn convert_symbol(code: SymbolCode) -> Option<&'static [u8]> {
	let kbd = unsafe { &KEYBOARD };
	let table = match kbd.shift_pressed() {
		true => &SYMBOL_SHIFT,
		false => &SYMBOL_PLAIN,
	};

	let idx = code.index() as usize;
	let ascii = table[idx];

	if kbd.control_pressed() && control_convertable(ascii) {
		let idx = ascii.wrapping_sub(b'@').wrapping_add(1) as usize;
		Some(&CONTROL[idx..=idx])
	} else {
		Some(&table[idx..=idx])
	}
}

fn convert_control(code: ControlCode) -> Option<&'static [u8]> {
	match code {
		ControlCode::Backspace => Some(b"\x7f"),
		ControlCode::Delete => Some(b"\x1b[3~"),
		ControlCode::Tab => Some(b"\x09"),
		ControlCode::Enter => Some(b"\x0d"),
		ControlCode::Escape => Some(b"\x1b"),
		_ => None,
	}
}

pub struct TTY {
	termios: Termios,
	console: Console,
	line_buffer: LineBuffer<4096>,
	into_process: VecDeque<u8>,
	session: Weak<Locked<Session>>,
	waitlist: WaitList,
	winsize: WinSize,
}

impl TTY {
	pub fn new(termios: Termios, winsize: WinSize) -> Self {
		Self {
			termios,
			line_buffer: LineBuffer::new(),
			into_process: VecDeque::new(),
			session: Weak::default(),
			console: Console::buffer_reserved(winsize),
			waitlist: WaitList::new(),
			winsize,
		}
	}

	pub fn connect(&mut self, sess: &Arc<Locked<Session>>) -> Result<(), Errno> {
		if let Some(_) = self.session.upgrade() {
			return Err(Errno::EPERM);
		}

		self.session = Arc::downgrade(sess);

		Ok(())
	}

	pub fn disconnect(&mut self) {
		self.session = Weak::default();
		self.line_buffer.clear();
		self.into_process.clear();
	}

	pub fn draw(&self) {
		self.console.draw();
	}

	fn input_convert<'a>(&self, data: Code, buf: &'a mut [u8]) -> Option<&'a [u8]> {
		let iconv = input_convert_const(data)?;

		for (i, c) in iconv.iter().enumerate() {
			buf[i] = *c;
		}

		if self.termios.iflag.contains(InputFlag::ICRNL) {
			buf.iter_mut().filter(|b| **b == CR).for_each(|b| *b = LF);
		}

		Some(&buf[0..iconv.len()])
	}

	fn output_convert<'a>(&self, buf: &'a mut [u8], len: usize) -> &'a [u8] {
		let mut cr_count = 0;

		if self
			.termios
			.oflag
			.contains(OutputFlag::OPOST | OutputFlag::ONLCR)
		{
			for i in (0..len).rev() {
				if buf[i] == LF {
					unsafe {
						let ptr = buf.as_mut_ptr();
						let src = ptr.offset(i as isize);
						let dst = src.offset(1);
						core::ptr::copy(src, dst, len - i);
					}
					buf[i] = CR;
					cr_count += 1;
				}
			}
		}
		&buf[0..(len + cr_count)]
	}

	/// echo back given characters.
	/// if character is non-printable,
	///   then escape with caret-notation to make it printable.
	///
	/// # caret-notation
	///
	/// represent non printable ascii `(0..=31, 127)` with `^('@' + ascii)` and MSB(bit 8) cleared.
	///
	/// ## examples
	/// - 0  (NUL) => `^@`
	/// - 1b (ESC) => `^[`
	/// - 7f (DEL) => `^?`
	fn do_echo(&mut self, buf: &[u8]) -> Result<(), NoSpace> {
		let c = buf[0];
		let echo_ctl = self.termios.lflag.contains(LocalFlag::ECHOCTL);
		let echo_e = self
			.termios
			.lflag
			.contains(LocalFlag::ECHOE | LocalFlag::ICANON);
		let echo_k = self
			.termios
			.lflag
			.contains(LocalFlag::ECHOK | LocalFlag::ICANON);
		let icanon = self.termios.lflag.contains(LocalFlag::ICANON);

		if let (DEL, true) = (c, echo_e) {
			self.console.write(&CURSOR[2]);
			self.console.write_one(c)?;
			return Ok(());
		}

		if let (NAK, true) = (c, echo_k) {
			self.console.write(b"\x1b[2K");
			self.console.write_one(CR)?;
			return Ok(());
		}

		for c in buf.iter().map(|b| *b) {
			if let (CR, true) | (LF, true) = (c, icanon) {
				self.console.write_one(c)?;
			} else {
				match (is_control(c), echo_ctl) {
					(true, true) => {
						self.console.write_one(b'^')?;
						self.console.write_one((b'@' + c) & !(1 << 7))?;
					}
					_ => self.console.write_one(c)?,
				}
			}
		}
		Ok(())
	}

	// NAK, '^U'
	// DEL, '^?'
	// EOF, '^D'
	fn do_icanon(&mut self, buf: &[u8], code: Code) {
		let c = buf[0];

		match c {
			DEL => self.line_buffer.backspace(),
			NAK => self.line_buffer.clear(),
			EOF => {}
			ESC => {
				let code = code.identify();
				let echo_ctl = self.termios.lflag.contains(LocalFlag::ECHOCTL);

				match (code, echo_ctl) {
					(KeyKind::Cursor(_), true) => {
						buf.iter().for_each(|b| self.line_buffer.put_char(*b))
					}
					(KeyKind::Cursor(c), false) => {
						self.console.write(&CURSOR[c.index() as usize]);
					}
					(_, _) => {}
				}
			}
			LF => {
				self.line_buffer.push(LF);
				self.into_process.extend(self.line_buffer.as_slice());
				self.line_buffer.clear();
			}
			_ => buf.iter().for_each(|b| self.line_buffer.put_char(*b)),
		}
	}

	fn send_signal(&self, c: u8) {
		use crate::process::signal::sig_code::SigCode;
		use crate::process::signal::sig_num::SigNum;

		// pr_debug!("tty: send signal: {}", c);

		let num = match c {
			x if x == self.termios.control_char[VINTR] => SigNum::INT,
			x if x == self.termios.control_char[VQUIT] => SigNum::QUIT,
			_ => unreachable!(),
		};

		let _ = send_signal_to_foreground(&self.session, num, SigCode::SI_KERNEL);
	}

	fn is_signal(&self, c: u8) -> bool {
		self.termios.lflag.contains(LocalFlag::ISIG)
			&& (self.termios.control_char[VINTR] == c || self.termios.control_char[VQUIT] == c)
	}
}

fn input_convert_const(code: Code) -> Option<&'static [u8]> {
	match code.identify() {
		KeyKind::Alpha(code) => convert_alpha(code),
		KeyKind::Symbol(code) => convert_symbol(code),
		KeyKind::Function(code) => convert_function(code),
		KeyKind::Keypad(code) => convert_keypad(code),
		KeyKind::Cursor(code) => convert_cursor(code),
		KeyKind::Control(code) => convert_control(code),
		KeyKind::Modifier(..) => None,
		KeyKind::Toggle(..) => None,
	}
}

/// from keyboard
impl ChWrite<Code> for TTY {
	/// # Safety
	///
	/// - context: irq_disabled: memory allocation, console lock
	fn write_one(&mut self, code: Code) -> Result<(), NoSpace> {
		let mut buf = [0; 16];
		let input = match self.input_convert(code, &mut buf) {
			None => return Ok(()),
			Some(i) => i,
		};

		if self.is_signal(input[0]) {
			// Isig
			self.send_signal(input[0]);
		} else {
			// Icanon
			if self.termios.lflag.contains(LocalFlag::ICANON) {
				self.do_icanon(input, code);
			} else {
				self.into_process.extend(input);
			}
		}

		// Opost & Onlcr
		let len = input.len();
		let output = self.output_convert(&mut buf, len);

		// Echo
		if self.termios.lflag.contains(LocalFlag::ECHO) {
			self.do_echo(output)?
		}

		// wake_up on event
		self.waitlist.wake_up_all();

		Ok(())
	}
}

/// from process
impl ChWrite<u8> for TTY {
	fn write_one(&mut self, data: u8) -> Result<(), NoSpace> {
		let mut buf = [data, 0];
		let iter = self.output_convert(&mut buf, 1);

		for c in iter.iter() {
			self.console.write_one(*c)?
		}

		if let Some(w) = Work::new_once(get_screen_draw_work()) {
			schedule_work(w);
		}

		Ok(())
	}
}

/// to process
impl ChRead<u8> for TTY {
	// Because memory allocator is not used in VecDeque.pop_front(),
	// this function don't need to be in irq_disabled context.
	fn read_one(&mut self) -> Option<u8> {
		self.into_process.pop_front()
	}
}

impl BlkWrite for TTY {}
impl BlkRead for TTY {}

#[derive(Clone)]
pub struct TTYFile {
	tty: Arc<Locked<TTY>>,
}

impl TTYFile {
	pub fn new(tty: Arc<Locked<TTY>>) -> Self {
		Self { tty }
	}

	pub fn lock_tty(&self) -> LockedGuard<'_, TTY> {
		self.tty.lock()
	}

	pub fn put_on_waitlist(&self) -> AtomicOps {
		let atomic = preempt_disable();
		self.lock_tty().waitlist.register();
		atomic
	}

	fn get_window_size(&self, argp: usize) -> Result<(), Errno> {
		let current = unsafe { CURRENT.get_ref() };
		let win_size = verify_ptr_mut::<WinSize>(argp, current)?;
		*win_size = self.tty.lock().winsize;

		Ok(())
	}

	fn get_foreground_group(&self, argp: usize) -> Result<(), Errno> {
		let current = unsafe { CURRENT.get_ref() };
		let foreground = verify_ptr_mut::<usize>(argp, current)?;
		*foreground = self
			.lock_tty()
			.session
			.upgrade()
			.and_then(|x| x.lock().foreground())
			.and_then(|x| x.upgrade())
			.map(|x| x.get_pgid().as_raw())
			.ok_or(Errno::ESRCH)?;

		Ok(())
	}

	fn set_foreground_group(&self, argp: usize) -> Result<(), Errno> {
		let current = unsafe { CURRENT.get_ref() };
		let pgid = Pgid::from(Pid::from_raw(*verify_ptr::<usize>(argp, current)?));

		self.lock_tty()
			.session
			.upgrade()
			.ok_or(Errno::ESRCH)
			.and_then(|x| x.lock().set_foreground(pgid))
	}

	fn get_termios(&self, argp: usize) -> Result<(), Errno> {
		let current = unsafe { CURRENT.get_ref() };

		let dst = verify_ptr_mut::<Termios>(argp, current)?;

		*dst = self.lock_tty().termios.clone();

		Ok(())
	}

	fn set_termios(&self, argp: usize) -> Result<(), Errno> {
		let current = unsafe { CURRENT.get_ref() };

		let src = verify_ptr::<Termios>(argp, current)?;

		self.lock_tty().termios = src.clone();

		Ok(())
	}

	fn set_ctty(&self, sess: &Arc<Locked<Session>>) -> Result<(), Errno> {
		let mut sess_lock = sess.lock();
		sess_lock.set_ctty(self.clone());
		self.lock_tty().connect(&sess)
	}

	fn release_ctty(&self, sess: &Arc<Locked<Session>>) -> Result<(), Errno> {
		let mut sess_lock = sess.lock();
		sess_lock.release_ctty();
		self.lock_tty().disconnect();
		Ok(())
	}
}

impl FileHandle for TTYFile {
	fn read(&self, buf: &mut [u8], io_flags: IOFlag) -> Result<usize, Errno> {
		let block = !io_flags.contains(IOFlag::O_NONBLOCK);
		let mut count = self.lock_tty().read(buf);
		while block && count == 0 {
			unsafe { poll_signal_queue()? };
			sleep_and_yield_atomic(Sleep::Light, self.put_on_waitlist());
			count += self.lock_tty().read(buf);
		}
		Ok(count)
	}

	fn write(&self, buf: &[u8], io_flags: IOFlag) -> Result<usize, Errno> {
		let block = !io_flags.contains(IOFlag::O_NONBLOCK);
		let mut count = self.lock_tty().write(buf);
		while block && count == 0 {
			unsafe { poll_signal_queue()? };
			sleep_and_yield_atomic(Sleep::Light, self.put_on_waitlist());
			count += self.lock_tty().write(buf);
		}
		Ok(count)
	}

	fn lseek(&self, _offset: isize, _whence: crate::fs::vfs::Whence) -> Result<usize, Errno> {
		Err(Errno::ESPIPE)
	}

	fn ioctl(&self, request: usize, argp: usize) -> Result<usize, Errno> {
		let current = unsafe { CURRENT.get_ref() };

		let rel = current
			.get_user_ext()
			.expect("must be user process")
			.lock_relation();
		let curr_sess = rel.get_session();

		if let Some(ref sess) = self.lock_tty().session.upgrade() {
			if !Arc::ptr_eq(sess, &curr_sess) {
				return Err(Errno::EPERM);
			}
		}

		match request as u32 {
			termios::TIOCGWINSZ => self.get_window_size(argp),
			termios::TIOCGPGRP => self.get_foreground_group(argp),
			termios::TIOCSPGRP => self.set_foreground_group(argp),
			termios::TIOCSCTTY => self.set_ctty(&curr_sess),
			termios::TIOCNOTTY => self.release_ctty(&curr_sess),
			termios::TCGETS => self.get_termios(argp),
			termios::TCSETSW | termios::TCSETS => self.set_termios(argp),
			x => {
				pr_warn!("tty: ioctl: unknown request: {}", x);
				Err(Errno::EINVAL)
			}
		}
		.map(|_| 0)
	}
}

fn is_printable(c: u8) -> bool {
	b' ' <= c && c <= b'~'
}

fn is_control(c: u8) -> bool {
	c < 0x20 || c == 0x7f
}

//! Editable line buffer

use alloc::boxed::Box;
pub struct LineBuffer<const CAP: usize> {
	buf: Box<[u8; CAP]>,
	cursor: usize,
	tail: usize,
}

impl<const CAP: usize> LineBuffer<CAP> {
	pub fn new() -> Self {
		Self {
			buf: unsafe { Box::new_zeroed().assume_init() },
			tail: 0,
			cursor: 0,
		}
	}

	pub fn full(&self) -> bool {
		self.tail >= CAP - 1
	}

	pub fn size(&self) -> usize {
		self.tail
	}

	pub fn empty(&self) -> bool {
		self.tail == 0
	}

	pub fn clear(&mut self) {
		self.tail = 0;
		self.cursor = 0;
	}

	/// Shift internal buffer's characters one by one from `begin` to `end` (inclusive range).
	/// characters **always** moved from `next` position to `current` position.
	///
	///
	/// # Examples
	///
	/// Where
	/// - `B`: `begin`
	/// - `E`: `end`
	/// - `C`: `current`
	/// - `N`: `next`
	///
	/// ## Case 1 (`B` = 0, `E` = 2)
	///
	/// ```
	/// initial state:   | iter 1:         | iter 2:
	///   B   E          |   B   E         |   B   E
	///   C N            |     C N         |       C N
	/// [ P Q R ? ? ? ]  | [ Q Q R ? ? ? ] | [ Q R R ? ? ? ]
	/// ```
	///
	/// ## Case 2 (`B` = 2, `E` = 0)
	/// ```
	/// initial state:   | iter 1:         | iter 2:
	///   E   B          |   E   B         |   E   B
	///     N C          |   N C           | N C
	/// [ P Q R ? ? ? ]  | [ P Q Q ? ? ? ] | [ P P Q ? ? ? ]
	/// ```
	fn shift_chars(&mut self, begin: isize, end: isize) {
		let direction = (end - begin).signum();

		let mut curr = begin;
		while curr != end {
			let next = curr + direction;
			self.buf[curr as usize] = self.buf[next as usize];
			curr = next;
		}
	}

	pub fn put_char(&mut self, c: u8) {
		if self.full() {
			return;
		}

		self.shift_chars(self.tail as isize, self.cursor as isize);
		self.buf[self.cursor] = c;
		self.cursor += 1;
		self.tail += 1;
	}

	pub fn push(&mut self, c: u8) {
		if self.tail == CAP {
			return;
		}

		self.buf[self.tail] = c;
		self.tail += 1;
	}

	pub fn backspace(&mut self) {
		if self.cursor == 0 {
			return;
		}

		self.cursor -= 1;
		self.tail -= 1;
		self.shift_chars(self.cursor as isize, self.tail as isize);
	}

	pub fn is_cursor_at_begin(&self) -> bool {
		self.cursor == 0
	}

	pub fn is_cursor_at_end(&self) -> bool {
		self.cursor == self.tail
	}

	pub fn cursor(&self) -> usize {
		self.cursor
	}

	pub fn move_cursor_left(&mut self) {
		if self.cursor != 0 {
			self.cursor -= 1;
		}
	}

	pub fn move_cursor_right(&mut self) {
		if self.cursor != self.tail {
			self.cursor += 1;
		}
	}

	pub fn move_cursor_head(&mut self) {
		self.cursor = 0;
	}

	pub fn move_cursor_tail(&mut self) {
		self.cursor = self.tail;
	}

	pub fn as_slice(&self) -> &[u8] {
		&self.buf[..self.tail]
	}
}

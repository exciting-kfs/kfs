//! Simple fixed-size circular queue.
//!
//! very similar to std::collections::VecDeque
//! but in out-of-space case,
//! this buffer will be overwrite data from olderest one.

use core::ops::{Index, IndexMut};

#[derive(PartialEq)]
enum State {
	Empty,
	Full,
	Avail,
}

pub struct WrapQueue<T, const N: usize> {
	data: [T; N],
	head: usize,
	tail: usize,
	state: State,
}

impl<T, const CAPACITY: usize> WrapQueue<T, CAPACITY> {
	/// construct new wrap_queue with value returned by `FnMut cb`
	pub fn from_fn<F>(cb: F) -> Self
	where
		F: FnMut(usize) -> T,
	{
		Self {
			data: core::array::from_fn(cb),
			head: 0,
			tail: 0,
			state: State::Empty,
		}
	}

	/// construct new wrap_queue contain N copies of `value`
	pub const fn with(value: T) -> Self
	where
		T: Copy,
	{
		Self {
			data: [value; CAPACITY],
			head: 0,
			tail: 0,
			state: State::Empty,
		}
	}

	/// translate linear index to discrete index.
	fn translate_idx(&self, idx: usize) -> Option<usize> {
		if idx >= self.size() {
			None
		} else {
			Some((self.head + idx) % CAPACITY)
		}
	}

	/// calculate occupied size
	pub fn size(&self) -> usize {
		match self.state {
			State::Full => CAPACITY,
			State::Empty => 0,
			State::Avail => {
				if self.head < self.tail {
					self.tail - self.head
				} else {
					self.tail + CAPACITY - self.head - 1
				}
			}
		}
	}

	fn circular_next(n: usize) -> usize {
		(n + 1) % CAPACITY
	}

	fn circular_prev(n: usize) -> usize {
		if n == 0 {
			CAPACITY - 1
		} else {
			n - 1
		}
	}

	pub fn empty(&self) -> bool {
		self.state == State::Empty
	}

	pub fn full(&self) -> bool {
		self.state == State::Full
	}

	/// access data by linear index.
	pub fn at_mut(&mut self, idx: usize) -> Option<&mut T> {
		self.translate_idx(idx).map(|i| &mut self.data[i])
	}

	pub fn at(&self, idx: usize) -> Option<&T> {
		self.translate_idx(idx).map(|i| &self.data[i])
	}

	/// push `n` copies of `item`
	pub fn push_n(&mut self, item: T, n: usize)
	where
		T: Copy,
	{
		for _ in 0..n {
			self.push(item);
		}
	}

	/// push data `n` times with default constructed value.
	pub fn reserve(&mut self, n: usize)
	where
		T: Default,
	{
		if n > CAPACITY {
			panic!("insufficient wrapqueue capacity.");
		}

		let size = self.size();
		for _ in size..n {
			let item = T::default();
			self.push(item);
		}
	}

	/// extend buffer but not initialize that space.
	fn extend(&mut self, n: usize) {
		for _ in 0..n {
			if self.full() {
				self.head = Self::circular_next(self.head);
			}

			self.tail = Self::circular_next(self.tail);

			self.state = match self.tail == self.head {
				true => State::Full,
				false => State::Avail,
			};
		}
	}

	pub fn push(&mut self, item: T) {
		self.data[self.tail] = item;
		self.extend(1)
	}

	pub fn pop(&mut self) -> Option<T>
	where
		T: Copy,
	{
		if self.empty() {
			return None;
		}

		let value = self.data[self.head];
		self.head = Self::circular_next(self.head);

		self.state = match self.tail == self.head {
			true => State::Empty,
			false => State::Avail,
		};

		Some(value)
	}

	/// create slice of wrap_queue.
	pub fn window<'a>(&'a self, start: usize, size: usize) -> Option<Window<&'a [T], CAPACITY>> {
		if size == 0 {
			return None;
		}

		let head = self.translate_idx(start)?;
		let tail = self.translate_idx(start + size - 1)? + 1;

		Some(Window {
			head,
			tail,
			data: &self.data,
		})
	}

	pub fn window_mut<'a>(
		&'a mut self,
		start: usize,
		size: usize,
	) -> Option<Window<&'a mut [T], CAPACITY>> {
		if size == 0 {
			return None;
		}

		let head = self.translate_idx(start)?;
		let tail = self.translate_idx(start + size - 1)? + 1;

		Some(Window {
			head,
			tail,
			data: &mut self.data,
		})
	}
}

/// sliced part of wrap_queue.
pub struct Window<T, const N: usize> {
	head: usize,
	tail: usize,
	data: T,
}

impl<'a, T, const CAPACITY: usize> Index<usize> for Window<&'a [T], CAPACITY> {
	type Output = T;

	fn index(&self, index: usize) -> &Self::Output {
		let index = (self.head + index) % CAPACITY;

		&self.data[index]
	}
}

impl<'a, T, const CAPACITY: usize> Index<usize> for Window<&'a mut [T], CAPACITY> {
	type Output = T;

	fn index(&self, index: usize) -> &Self::Output {
		let index = (self.head + index) % CAPACITY;

		&self.data[index]
	}
}

impl<'a, T, const CAPACITY: usize> IndexMut<usize> for Window<&'a mut [T], CAPACITY> {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		let index = (self.head + index) % CAPACITY;

		&mut self.data[index]
	}
}

/// create iterator of linearly alligned wrap_queue's raw memory representation.
impl<'a, T, const N: usize> IntoIterator for Window<&'a [T], N> {
	type Item = &'a [T];
	type IntoIter = core::array::IntoIter<Self::Item, 2>;

	fn into_iter(self) -> Self::IntoIter {
		match self.head < self.tail {
			true => [&self.data[self.head..self.tail], &[]],
			false => [&self.data[self.head..], &self.data[0..self.tail]],
		}
		.into_iter()
	}
}

//! Simple fixed-size circular queue.
//!
//! very similar to std::collections::VecDeque
//! but in out-of-space case,
//! this buffer will be overwrite data from olderest one.

use core::mem::MaybeUninit;
use core::ops::{Index, IndexMut};
use core::slice::{from_raw_parts, from_raw_parts_mut};

use alloc::boxed::Box;

#[derive(PartialEq)]
enum State {
	Empty,
	Full,
	Avail,
}

pub struct WrapQueue<T, const N: usize> {
	data: Box<[MaybeUninit<T>]>,
	head: usize,
	tail: usize,
	state: State,
}

impl<T, const CAPACITY: usize> WrapQueue<T, CAPACITY> {
	pub fn new() -> Self {
		Self {
			data: Box::new_uninit_slice(CAPACITY),
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
					(CAPACITY - self.head) + self.tail
				}
			}
		}
	}

	pub fn reset(&mut self) {
		self.tail = self.head;
		self.state = State::Empty;
	}

	pub fn empty(&self) -> bool {
		self.state == State::Empty
	}

	pub fn full(&self) -> bool {
		self.state == State::Full
	}

	/// access data by linear index.
	pub fn at_mut(&mut self, idx: usize) -> Option<&mut T> {
		self.translate_idx(idx)
			.map(|i| unsafe { self.data[i].assume_init_mut() })
	}

	pub fn at(&self, idx: usize) -> Option<&T> {
		self.translate_idx(idx)
			.map(|i| unsafe { self.data[i].assume_init_ref() })
	}

	/// push `n` copies of `item`
	pub fn push_copies(&mut self, item: T, n: usize)
	where
		T: Clone,
	{
		for _ in 0..n {
			self.push(item.clone());
		}
	}

	/// push data `n` times with default constructed value.
	pub fn push_defaults(&mut self, n: usize)
	where
		T: Default,
	{
		for _ in 0..n {
			self.push(T::default());
		}
	}

	fn increase_size(&mut self, n: usize) {
		if self.size() + n >= CAPACITY {
			self.tail = (self.tail + n) % CAPACITY;
			self.head = self.tail;
			self.state = State::Full;
		} else {
			self.tail = (self.tail + n) % CAPACITY;
			self.state = State::Avail;
		}
	}

	fn decrease_size(&mut self, n: usize) {
		if self.size() <= n {
			self.head = (self.head + n) % CAPACITY;
			self.tail = self.head;
			self.state = State::Empty;
		} else {
			self.head = (self.head + n) % CAPACITY;
			self.state = State::Avail;
		}
	}

	pub fn push(&mut self, item: T) {
		self.data[self.tail] = MaybeUninit::new(item);
		self.increase_size(1);
	}

	pub fn pop(&mut self) -> Option<T> {
		if self.empty() {
			return None;
		}

		let val = unsafe { self.data[self.head].assume_init_read() };
		self.decrease_size(1);

		Some(val)
	}

	fn index_as_ptr(&self, raw_idx: usize) -> *const T {
		((&self.data[raw_idx]) as *const MaybeUninit<T>).cast()
	}

	fn index_as_mut_ptr(&mut self, raw_idx: usize) -> *mut T {
		((&mut self.data[raw_idx]) as *mut MaybeUninit<T>).cast()
	}

	pub fn as_slices(&self, start: usize, len: usize) -> Option<[&'_ [T]; 2]> {
		if self.size() < start + len {
			return None;
		}

		let head = (self.head + start) % CAPACITY;
		let tail = (head + len) % CAPACITY;

		let lstart = self.index_as_ptr(head);
		let rstart = self.index_as_ptr(0);

		let (lsize, rsize) = match head < tail {
			true => (tail - head, 0),
			false => (CAPACITY - head, tail),
		};

		Some(unsafe { [from_raw_parts(lstart, lsize), from_raw_parts(rstart, rsize)] })
	}

	pub fn as_slices_mut(&mut self, start: usize, len: usize) -> Option<[&'_ mut [T]; 2]> {
		if self.size() < start + len {
			return None;
		}

		let head = (self.head + start) % CAPACITY;
		let tail = (head + len) % CAPACITY;

		let lstart = self.index_as_mut_ptr(head);
		let rstart = self.index_as_mut_ptr(0);

		let (lsize, rsize) = match head < tail {
			true => (tail - head, 0),
			false => (CAPACITY - head, tail),
		};

		Some(unsafe {
			[
				from_raw_parts_mut(lstart, lsize),
				from_raw_parts_mut(rstart, rsize),
			]
		})
	}

	pub fn read(&mut self, buf: &mut [T]) -> usize {
		let old_size = self.size();
		let dec_size = old_size.min(buf.len());

		let slices = self.as_slices(0, dec_size).unwrap();

		let mut remain = dec_size;
		let mut cursor = 0;
		for chunk in slices {
			if remain == 0 {
				break;
			}

			unsafe {
				buf.as_mut_ptr()
					.add(cursor)
					.copy_from_nonoverlapping(chunk.as_ptr(), chunk.len())
			};

			remain -= chunk.len();
			cursor += chunk.len();
		}

		self.decrease_size(dec_size);

		dec_size
	}

	pub fn write(&mut self, buf: &[T]) -> usize {
		let old_size = self.size();
		let inc_size = (CAPACITY - old_size).min(buf.len());

		self.increase_size(inc_size);

		let slices = self.as_slices_mut(old_size, inc_size).unwrap();

		let mut remain = inc_size;
		let mut cursor = 0;
		for chunk in slices {
			if remain == 0 {
				break;
			}

			unsafe {
				buf.as_ptr()
					.add(cursor)
					.copy_to_nonoverlapping(chunk.as_mut_ptr(), chunk.len())
			};

			remain -= chunk.len();
			cursor += chunk.len();
		}

		inc_size
	}

	pub fn capacity() -> usize {
		CAPACITY
	}

	/// create slice of wrap_queue.
	pub fn window<'a>(&'a self, start: usize, len: usize) -> Option<Window<'a, T, CAPACITY>> {
		if self.size() < start + len {
			return None;
		}

		Some(Window {
			inner: self,
			start,
			len,
		})
	}

	pub fn window_mut<'a>(
		&'a mut self,
		start: usize,
		len: usize,
	) -> Option<WindowMut<'a, T, CAPACITY>> {
		if self.size() < start + len {
			return None;
		}

		Some(WindowMut {
			inner: self,
			start,
			len,
		})
	}
}

impl<T, const CAPACITY: usize> Index<usize> for WrapQueue<T, CAPACITY> {
	type Output = T;

	fn index(&self, index: usize) -> &Self::Output {
		self.at(index).expect("WrapQueue: index out of bound")
	}
}

impl<T, const CAPACITY: usize> IndexMut<usize> for WrapQueue<T, CAPACITY> {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		self.at_mut(index).expect("WrapQueue: index out of bound")
	}
}

/// sliced part of wrap_queue.
pub struct Window<'a, T, const CAP: usize> {
	inner: &'a WrapQueue<T, CAP>,
	start: usize,
	len: usize,
}

impl<'a, T, const CAP: usize> Window<'a, T, CAP> {
	pub fn as_slices(&'a self) -> [&'a [T]; 2] {
		self.inner.as_slices(self.start, self.len).unwrap()
	}
}

impl<'a, T, const CAP: usize> Index<usize> for Window<'a, T, CAP> {
	type Output = T;

	fn index(&self, index: usize) -> &Self::Output {
		&self.inner[self.start + index]
	}
}

pub struct WindowMut<'a, T, const CAP: usize> {
	inner: &'a mut WrapQueue<T, CAP>,
	start: usize,
	len: usize,
}

impl<'a, T, const CAP: usize> WindowMut<'a, T, CAP> {
	pub fn as_slices(&'a self) -> [&'a [T]; 2] {
		self.inner.as_slices(self.start, self.len).unwrap()
	}

	pub fn as_slices_mut(&'a mut self) -> [&'a mut [T]; 2] {
		self.inner.as_slices_mut(self.start, self.len).unwrap()
	}
}

impl<'a, T, const CAP: usize> Index<usize> for WindowMut<'a, T, CAP> {
	type Output = T;

	fn index(&self, index: usize) -> &Self::Output {
		&self.inner[self.start + index]
	}
}

impl<'a, T, const CAP: usize> IndexMut<usize> for WindowMut<'a, T, CAP> {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		&mut self.inner[self.start + index]
	}
}

mod test {
	use core::array;

	use alloc::collections::VecDeque;
	use kfs_macro::ktest;

	use crate::util::lcg::LCG;

	use super::*;

	type RingBuffer<const CAP: usize> = WrapQueue<u8, CAP>;

	#[ktest(wrap_queue)]
	pub fn basic() {
		let mut buf: RingBuffer<2> = RingBuffer::new();

		assert_eq!(buf.empty(), true);
		assert_eq!(buf.full(), false);
		assert_eq!(buf.size(), 0);
		assert_eq!(buf.pop(), None);

		buf.push(42);
		assert_eq!(buf.empty(), false);
		assert_eq!(buf.full(), false);
		assert_eq!(buf.size(), 1);

		buf.push(42);
		assert_eq!(buf.empty(), false);
		assert_eq!(buf.full(), true);
		assert_eq!(buf.size(), 2);
	}

	#[ktest(wrap_queue)]
	pub fn wrap() {
		let mut buf: RingBuffer<4> = RingBuffer::new();

		for i in 0..4 {
			buf.push(i);
		}

		assert_eq!(buf.full(), true);
		assert_eq!(buf.empty(), false);
		assert_eq!(buf.size(), 4);

		{
			let win = buf.window(0, 4).unwrap();
			for i in 0..4 {
				assert_eq!(win[i], i as u8);
			}
		}

		{
			assert_eq!(buf.window(0, 5).is_none(), true);
			assert_eq!(buf.window(1, 4).is_none(), true);
			assert_eq!(buf.window(5, 1).is_none(), true);
		}

		buf.push(42);

		assert_eq!(buf.full(), true);
		assert_eq!(buf.empty(), false);
		assert_eq!(buf.size(), 4);

		assert_eq!(buf.pop(), Some(1));
		assert_eq!(buf.pop(), Some(2));
		assert_eq!(buf.pop(), Some(3));
		assert_eq!(buf.pop(), Some(42));
		assert_eq!(buf.pop(), None);

		assert_eq!(buf.full(), false);
		assert_eq!(buf.empty(), true);
	}

	#[ktest(wrap_queue)]
	pub fn push_multiple() {
		let mut buf: RingBuffer<4> = RingBuffer::new();

		buf.push_copies(42, 4);

		for _ in 0..4 {
			assert_eq!(buf.pop(), Some(42));
		}

		assert_eq!(buf.size(), 0);

		buf.push_defaults(4);
		for _ in 0..4 {
			assert_eq!(buf.pop(), Some(0));
		}
	}

	#[ktest(wrap_queue)]
	pub fn wrap_heavy() {
		const BUFFER_SIZE: usize = 4096;
		fn push_dq(v: &mut VecDeque<u32>, val: u32) {
			if v.len() == BUFFER_SIZE {
				v.pop_front();
			}
			v.push_back(val);
		}

		let mut dq: VecDeque<u32> = VecDeque::new();
		let mut wq: WrapQueue<u32, BUFFER_SIZE> = WrapQueue::new();

		let mut lcg = LCG::new(42);
		let mut random = |min: u32, max: u32| min + (lcg.rand() % (max - min));

		let mut id = 0;
		for _ in 0..100 {
			for _ in 0..random(0, 4096) {
				push_dq(&mut dq, id);
				wq.push(id);

				id += 1;
			}
			assert_eq!(wq.size(), dq.len());
			for i in 0..wq.size() {
				assert_eq!(wq[i], dq[i]);
			}
		}
	}

	#[ktest(wrap_queue)]
	pub fn io_basic() {
		let mut wq: WrapQueue<u8, 4096> = WrapQueue::new();
		let mut buf: [u8; 256] = array::from_fn(|i| i as u8);

		let ret = wq.write(&buf);

		assert_eq!(wq.size(), 256);
		assert_eq!(ret, 256);

		for i in 0..256 {
			assert_eq!(wq[i], i as u8);
		}

		wq.reset();
		wq.push_copies(42, 128);

		let ret = wq.read(&mut buf);
		assert_eq!(ret, 128);

		for i in 0..128 {
			assert_eq!(buf[i], 42);
		}
	}

	#[ktest(wrap_queue)]
	pub fn io_heavy() {
		const BUFFER_SIZE: usize = 1024;

		fn dq_read(dq: &mut VecDeque<u8>, buf: &mut [u8]) -> usize {
			let len = dq.len().min(buf.len());

			for x in 0..len {
				buf[x] = dq.pop_front().unwrap();
			}

			len
		}

		fn dq_write(dq: &mut VecDeque<u8>, buf: &[u8]) -> usize {
			let len = (BUFFER_SIZE - dq.len()).min(buf.len());

			for x in 0..len {
				dq.push_back(buf[x]);
			}

			len
		}

		let mut buf_a: [u8; 256] = [0; 256];
		let mut buf_b: [u8; 256] = [0; 256];

		let mut dq: VecDeque<u8> = VecDeque::new();
		let mut wq: WrapQueue<u8, BUFFER_SIZE> = WrapQueue::new();

		let mut lcg = LCG::new(42);
		let mut random = |min: u32, max: u32| min + (lcg.rand() % (max - min));

		for _ in 0..500 {
			let sz = random(0, 257) as usize;

			if random(0, 2) == 0 {
				buf_a.fill(sz as u8);
				buf_b.fill(sz as u8);

				let len_a = dq_write(&mut dq, &buf_a[0..sz]);
				let len_b = wq.write(&buf_b[0..sz]);

				assert_eq!(len_a, len_b);
				assert_eq!(dq.len(), wq.size());
				for i in 0..dq.len() {
					assert_eq!(dq[i], wq[i]);
				}
			} else {
				let len_a = dq_read(&mut dq, &mut buf_a[0..sz]);
				let len_b = wq.read(&mut buf_b[0..sz]);
				assert_eq!(len_a, len_b);
				assert_eq!(buf_a, buf_b);
			}
		}
	}
}

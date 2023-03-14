pub enum Error {
	Empty,
	Full
}

pub struct SizedStack<T, const CAP: usize> {
	buf: [T; CAP],
	top: usize
}

impl<T, const CAP: usize> SizedStack<T, CAP>
where T: Default
{
	pub fn from_fn<F>(cb: F) -> Self
	where F: FnMut(usize) -> T
	{
		SizedStack {
			buf: core::array::from_fn(cb),
			top: 0
		}
	}

	pub fn filled<F>(cb: F) -> Self
	where F: FnMut(usize) -> T
	{
		SizedStack {
			buf: core::array::from_fn(cb),
			top: CAP
		}
	}

	pub fn push(&mut self, data: T) -> Result<(), Error> {
		if self.top == CAP {
			Err(Error::Full)
		} else {
			self.buf[self.top] = data;
			self.top += 1;
			Ok(())
		}
	}


	pub fn pop(&mut self) -> Result<T, Error> {
		if self.top == 0 {
			Err(Error::Empty)
		} else {
			let target = &mut self.buf[self.top - 1];
			let poped = core::mem::take(target);
			self.top -= 1;
			Ok(poped)
		}
	}

	pub fn iter(&self)-> Iter<'_, T, CAP> {
		Iter::new(self)
	}
}

impl<'a, T: Default, const CAP: usize> IntoIterator for &'a SizedStack<T, CAP> {
	type Item = &'a T;
	type IntoIter = Iter<'a, T, CAP>;
	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub struct Iter <'a, T, const CAP: usize> {
	idx: usize,
	buf: &'a [T]
}

impl<'a, T, const CAP: usize> Iter<'a, T, CAP> {
	fn new(contatiner: &'a SizedStack<T, CAP>) -> Self {
		Iter {
			idx: 0,
			buf: &contatiner.buf
		}
	}
}

impl<'a, T: Default, const CAP: usize> Iterator for Iter<'a, T, CAP> {
	type Item = &'a T;
	fn next(&mut self) -> Option<Self::Item> {
		let idx = self.idx;

		(idx < CAP).then(|| {
			let data = &self.buf[idx];
			self.idx += 1;
			data
		})
	}
}

pub enum Error {
	Empty,
	DoubleFree,
	OutOfRange,
}

pub struct Recycle<const CAP: usize> {
	bufctl: [usize; CAP],
	free: usize
}

const NONE: usize = usize::MAX;

impl<const CAP: usize> Recycle<CAP> {
	pub fn new() -> Self {
		Recycle {
			bufctl: core::array::from_fn(|idx| idx + 1),
			free: 0
		}
	}

	pub fn alloc(&mut self) -> Result<usize, Error> {
		let free = self.free;

		if free >= CAP {
			Err(Error::Empty)
		} else {
			self.free = self.bufctl[free];
			self.bufctl[free] = NONE;
			Ok(free)
		}
	}


	pub fn free(&mut self, idx: usize) -> Result<(), Error> {
		if idx >= CAP {
			Err(Error::OutOfRange)
		} else if self.bufctl[idx] != NONE {
			Err(Error::DoubleFree)
		} else {
			self.bufctl[idx] = self.free;
			self.free = idx;
			Ok(())
		}
	}

	pub fn iter(&self) -> Iter<'_, CAP> {
		Iter::new(self)
	}
}

impl<'a, const CAP: usize> IntoIterator for &'a Recycle<CAP> {
	type Item = &'a usize;
	type IntoIter = Iter<'a, CAP>;
	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub struct Iter <'a, const CAP: usize> {
	idx: usize,
	buf: &'a [usize]
}

impl<'a, const CAP: usize> Iter<'a, CAP> {
	fn new(contatiner: &'a Recycle<CAP>) -> Self {
		Iter {
			idx: 0,
			buf: &contatiner.bufctl
		}
	}
}

impl<'a, const CAP: usize> Iterator for Iter<'a, CAP> {
	type Item = &'a usize;
	fn next(&mut self) -> Option<Self::Item> {
		let idx = self.idx;

		(idx < CAP).then(|| {
			let data = &self.buf[idx];
			self.idx += 1;
			data
		})
	}
}

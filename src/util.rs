pub struct LazyInit<T> {
	value: Option<T>,
	init: fn() -> T,
}

impl<T> LazyInit<T> {
	pub const fn new(cb: fn() -> T) -> Self {
		LazyInit {
			value: None,
			init: cb,
		}
	}

	pub fn get(&mut self) -> &mut T {
		if let None = self.value {
			self.value = Some((self.init)())
		}

		self.value.as_mut().unwrap()
	}
}

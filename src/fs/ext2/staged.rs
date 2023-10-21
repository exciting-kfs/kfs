use alloc::boxed::Box;

pub struct Staged<T = (), U = ()> {
	modify: Box<dyn FnMut(T) -> U>,
	restore: Option<Box<dyn FnMut() -> ()>>,
}

impl Staged<(), ()> {
	pub fn new<F: FnMut(()) -> () + 'static>(modify: F) -> Self {
		Self {
			modify: Box::new(modify),
			restore: None,
		}
	}
}

impl<T, U> Staged<T, U> {
	pub fn func<F: FnMut(T) -> U + 'static>(modify: F) -> Self {
		Self {
			modify: Box::new(modify),
			restore: None,
		}
	}

	pub fn func_with_restore<F: FnMut(T) -> U + 'static, R: FnMut() -> () + 'static>(
		modify: F,
		restore: R,
	) -> Self {
		Self {
			modify: Box::new(modify),
			restore: Some(Box::new(restore)),
		}
	}

	pub fn commit(mut self, param: T) -> U {
		self.restore = None;

		(self.modify)(param)
	}
}

impl<T, U> Drop for Staged<T, U> {
	fn drop(&mut self) {
		if let Some(restore) = self.restore.as_mut() {
			(restore)()
		}
	}
}

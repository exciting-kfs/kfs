use alloc::boxed::Box;

pub struct Staged<T = (), U = ()> {
	modify: Box<dyn FnMut(T) -> U>,
}

impl Staged<(), ()> {
	pub fn new<F: FnMut(()) -> () + 'static>(modify: F) -> Self {
		Self {
			modify: Box::new(modify),
		}
	}
}

impl<T, U> Staged<T, U> {
	pub fn func<F: FnMut(T) -> U + 'static>(modify: F) -> Self {
		Self {
			modify: Box::new(modify),
		}
	}

	pub fn commit(&mut self, param: T) -> U {
		(self.modify)(param)
	}
}

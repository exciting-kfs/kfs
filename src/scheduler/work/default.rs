use alloc::boxed::Box;

use crate::sync::LocalLocked;

use super::{Error, Workable};

pub struct DefaultWork<ArgType> {
	func: fn(&mut ArgType) -> Result<(), Error>,
	arg: LocalLocked<Box<ArgType>>,
}

impl<ArgType> DefaultWork<ArgType> {
	pub fn new(func: fn(&mut ArgType) -> Result<(), Error>, arg: Box<ArgType>) -> Self {
		Self {
			func,
			arg: LocalLocked::new(arg),
		}
	}
}

impl<ArgType> Workable for DefaultWork<ArgType> {
	fn work(&self) -> Result<(), Error> {
		(self.func)(self.arg.lock().as_mut())
	}
}

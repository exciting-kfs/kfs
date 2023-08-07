use core::{mem::size_of, sync::atomic::Ordering};

use crate::{interrupt::InterruptFrame, process::task::CURRENT, register, RUN_TIME};

use super::{
	kernel_stack_bottom,
	stackframe::{self, Stackframe},
};

/// The type that holds the top most base pointer of the generated context.
pub struct StackDump {
	begin: usize,
}

impl StackDump {
	#[inline(never)]
	pub fn new() -> StackDump {
		let bp = register!("ebp");
		let bp = stackframe::next(bp);

		StackDump { begin: bp }
	}

	pub fn iter(&self) -> Iter {
		Iter::new(self.begin as usize)
	}
}

impl IntoIterator for StackDump {
	type Item = Stackframe;
	type IntoIter = Iter;
	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub struct Iter {
	base: usize,
	end: usize,
}

impl Iter {
	fn new(base: usize) -> Self {
		let end = if RUN_TIME.load(Ordering::Relaxed) {
			let current = unsafe { CURRENT.get_mut() };
			let stack_base = current.kstack_base();
			let user = current.get_user_ext().is_some();
			if user {
				stack_base - size_of::<InterruptFrame>() - size_of::<usize>() * 2
			} else {
				stack_base - size_of::<usize>() * 4
			}
		} else {
			kernel_stack_bottom as usize
		};

		Self { base, end }
	}
}

impl Iterator for Iter {
	type Item = Stackframe;
	fn next(&mut self) -> Option<Self::Item> {
		if self.base == self.end {
			return None;
		}

		let ret = Some(Stackframe::new(self.base));
		self.base = stackframe::next(self.base);
		ret
	}
}

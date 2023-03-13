use super::stackframe_iter::StackframeIter;
use crate::register;

use super::stackframe::{self, Stackframe};

/// The type that holds the top most base pointer of the generated context.
pub struct StackDump {
	begin: *const usize,
}

impl StackDump {
	#[inline(never)]
	pub fn new() -> StackDump {
		let bp = register!("ebp") as *const usize;
		let bp = stackframe::next(bp);

		StackDump { begin: bp }
	}

	pub fn iter(&self) -> StackframeIter {
		self.into_iter()
	}
}

impl IntoIterator for StackDump {
	type IntoIter = StackframeIter;
	type Item = Stackframe;
	fn into_iter(self) -> Self::IntoIter {
		StackframeIter {
			base_ptr: self.begin,
		}
	}
}

impl IntoIterator for &StackDump {
	type IntoIter = StackframeIter;
	type Item = Stackframe;
	fn into_iter(self) -> Self::IntoIter {
		StackframeIter {
			base_ptr: self.begin,
		}
	}
}

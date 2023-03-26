use super::stackframe::{self, Stackframe};

extern "C" {
	/// Do not __call__ this function. it's not function at all.
	/// but just pointer, which points bottom of the kernel stack.
	fn kernel_stack_bottom();
}

pub struct StackframeIter {
	pub(super) base_ptr: *const usize,
}

impl Iterator for StackframeIter {
	type Item = Stackframe;
	fn next(&mut self) -> Option<Self::Item> {
		let stack_base = kernel_stack_bottom as *const usize;
		if self.base_ptr == stack_base {
			return None;
		}

		let ret = Some(Stackframe::new(self.base_ptr));
		self.base_ptr = stackframe::next(self.base_ptr);
		ret
	}
}

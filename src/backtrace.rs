//! Backtrace

mod register;
mod stack_dump;
mod stackframe;

use rustc_demangle::demangle;

extern "C" {
	/// Do not __call__ this function. it's not function at all.
	/// but just pointer, which points bottom of the kernel stack.
	pub fn kernel_stack_bottom();

	pub fn kernel_stack_top();
}

use crate::boot;
use crate::pr_info;

pub use stack_dump::StackDump;

pub struct Backtrace {
	stack: StackDump,
}

impl Backtrace {
	pub fn new(stack: StackDump) -> Self {
		Backtrace { stack }
	}

	/// Print call stack trace of `StackDump`.
	pub fn print_trace(&self) {
		for (idx, frame) in self.stack.iter().enumerate() {
			let ksyms = &boot::get_ksyms();

			let name = ksyms
				.find_name_by_addr(frame.fn_addr)
				.unwrap_or_else(|| "<unknown>");

			pr_info!("frame #{}: {:?}: {:?}", idx, frame.fn_addr, demangle(name));
		}
	}
}

/// Print call stack trace in current context.
#[macro_export]
macro_rules! print_stacktrace {
	() => {
		let dump = $crate::backtrace::StackDump::new();
		let bt = $crate::backtrace::Backtrace::new(dump);
		bt.print_trace();
	};
}

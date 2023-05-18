//! Backtrace

mod register;
mod stack_dump;
mod stackframe;
mod stackframe_iter;

use rustc_demangle::demangle;

use crate::{boot::BOOT_INFO, pr_info};

pub use stack_dump::StackDump;

pub struct Backtrace {
	stack: StackDump,
}

impl Backtrace {
	pub fn new(stack: StackDump) -> Self {
		Backtrace { stack }
	}

	/// Print call stack trace of StackDump.
	pub fn print_trace(&self) {
		for (idx, frame) in self.stack.iter().enumerate() {
			let ksyms = &BOOT_INFO.lock().ksyms;

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

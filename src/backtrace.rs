//! Backtrace

mod register;
mod stack_dump;
mod stackframe;
mod stackframe_iter;

use rustc_demangle::demangle;

use crate::{pr_info, boot::{SYMTAB, STRTAB}};

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
			let name = self.find_name(frame.fn_addr).unwrap_or_default();
			pr_info!("frame #{}: {:?}: {:?}", idx, frame.fn_addr, demangle(name));
		}
	}

	/// Find function name using Symtab and Strtab
	fn find_name(&self, fn_addr: *const usize) -> Option<&'static str> {
		unsafe {
			let index = SYMTAB.get_name_index(fn_addr)?;
			let name = STRTAB.get_name(index);
			name
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

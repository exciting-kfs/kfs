//! Utils for running test
//!
//! To mark normal function as test function, use \#\[ktest\] attribute.
//! That will automatically create `static TestCase` variable for that function,
//! and link against .test_array section.
//! In .test_array section, there is linker provided start and end of section symbol
//! and `TEST_ARRAY` is holding them.
//! Since all objects in .test_array section are have same type. as well as size and alignment.
//! so we can use that section just like array (or slice).

use core::{mem::MaybeUninit, slice};

extern "Rust" {
	/// Begining of the .test_array section.
	/// link_name must be same as linker's one.
	#[link_name = "__test_array_start"]
	static TEST_ARRAY_START: MaybeUninit<TestCase>;

	/// End of the .test_array section.
	/// link_name must be same as linker's one.
	#[link_name = "__test_array_end"]
	static TEST_ARRAY_END: MaybeUninit<TestCase>;
}

/// Holds all test cases.
/// Note this struct does not provide constructor is intended.
pub struct TestArray {
	start: *const TestCase,
	end: *const TestCase,
}

/// Represent test function and it's name
pub struct TestCase {
	name: &'static str,
	func: fn(),
}

unsafe impl Sync for TestArray {}
unsafe impl Send for TestArray {}

/// Single instance of TestArray.
pub static TEST_ARRAY: TestArray = unsafe {
	TestArray {
		start: TEST_ARRAY_START.as_ptr(),
		end: TEST_ARRAY_END.as_ptr(),
	}
};

impl TestArray {
	/// Since slice is cannot created at const context, create it lazily.
	pub fn as_slice(&self) -> &'static [TestCase] {
		unsafe {
			let len = { self.end.offset_from(self.start) } as usize;
			return slice::from_raw_parts(self.start, len);
		}
	}
}

impl TestCase {
	pub const fn new(name: &'static str, func: fn()) -> Self {
		Self { name, func }
	}

	pub fn run(&self) {
		(self.func)();
	}

	pub fn get_name(&self) -> &str {
		self.name
	}
}

/// After test is ended. we have to shutdown QEMU VM.
/// and this will do that with QEMU's special device (isa-debug-exit)
pub fn exit_qemu_with(code: u32) -> ! {
	crate::io::pmio::Port::new(0x501).write_u32(code);
	unreachable!();
}

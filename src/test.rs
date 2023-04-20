use core::{mem::MaybeUninit, slice};

use crate::pr_info;

extern "Rust" {
	#[link_name = "__test_array_start"]
	static TEST_ARRAY_START: MaybeUninit<TestCase>;

	#[link_name = "__test_array_end"]
	static TEST_ARRAY_END: MaybeUninit<TestCase>;

}

pub struct TestArray {
	start: *const TestCase,
	end: *const TestCase,
}

pub struct TestCase {
	name: &'static str,
	func: fn(),
}

unsafe impl Sync for TestArray {}
unsafe impl Send for TestArray {}

#[link_section = ".test_array"]
static PHANTOM: [TestCase; 0] = [];

#[no_mangle]
#[allow(unused_must_use)]
fn __test_array_used() {
	PHANTOM.len();
}

pub static TEST_ARRAY: TestArray = unsafe {
	TestArray {
		start: TEST_ARRAY_START.as_ptr(),
		end: TEST_ARRAY_END.as_ptr(),
	}
};

impl TestArray {
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
		pr_info!("Testing: {}", self.name);
		(self.func)();
		pr_info!("...\x1b[32mok\x1b[39m");
	}
}

use core::{marker::PhantomData, mem::size_of, ops::Deref, slice};

extern "Rust" {
	#[link_name = "__test_start"]
	static TEST_FUNCTION_START: ();

	#[link_name = "__test_end"]
	static TEST_FUNCTION_END: ();
}

pub struct RawSlice<T> {
	start: &'static (),
	end: &'static (),
	_p: PhantomData<T>,
}

pub static TEST_FUNCTION_ARRAY: RawSlice<fn() -> ()> = unsafe {
	RawSlice {
		start: &TEST_FUNCTION_START,
		end: &TEST_FUNCTION_END,
		_p: PhantomData,
	}
};

impl<T> Deref for RawSlice<T> {
	type Target = [T];

	fn deref(&self) -> &Self::Target {
		let len = (self.end as *const _ as usize - self.start as *const _ as usize)
			/ size_of::<*const T>();
		unsafe { slice::from_raw_parts(self.start as *const _ as usize as *const T, len) }
	}
}

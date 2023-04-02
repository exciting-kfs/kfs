use core::ffi::{c_char, CStr};

/// String table in the '.strtab' section, used to get a symbol name.
pub struct Strtab {
	addr: *const u8,
}

impl Strtab {
	pub const fn new() -> Self {
		Strtab { addr: 0 as *const u8 }
	}

	pub fn init(addr: *const u8) -> Self {
		Strtab { addr }
	}

	/// Get the name formed C style string and transform to a string slice.
	pub fn get_name(&self, index: Option<isize>) -> Option<&'static str> {
		index.and_then(|idx| {
			let start = unsafe { self.addr.offset(idx) } as *const c_char;
			unsafe { CStr::from_ptr(start).to_str().ok() }
		})
	}
}

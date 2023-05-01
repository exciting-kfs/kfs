use core::{
	ffi::{c_char, CStr},
	marker::PhantomData,
};

/// String table in the '.strtab' section, used to get a symbol name.
pub struct Strtab {
	addr: *const c_char,
	size: usize,
}

impl Strtab {
	pub fn new(addr: *const c_char, size: usize) -> Self {
		Strtab { addr, size }
	}

	pub fn addr(&self) -> *const c_char {
		self.addr
	}

	/// Get the name formed C style string and transform to a string slice.
	pub fn get_name(&self, index: isize) -> Option<&'static str> {
		let start = unsafe { self.addr.offset(index) };
		unsafe { CStr::from_ptr(start).to_str().ok() }
	}

	pub fn iter(&self) -> Iter {
		Iter::new(self)
	}
}

impl<'a> IntoIterator for &'a Strtab {
	type Item = &'a str;
	type IntoIter = Iter<'a>;
	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub struct Iter<'a> {
	ptr: *const c_char,
	end: *const c_char,
	_p: PhantomData<&'a c_char>,
}

impl<'a> Iter<'a> {
	fn new(cont: &Strtab) -> Self {
		Iter {
			ptr: cont.addr,
			end: unsafe { cont.addr.add(cont.size) },
			_p: PhantomData,
		}
	}
}

impl<'a> Iterator for Iter<'a> {
	type Item = &'a str;
	fn next(&mut self) -> Option<Self::Item> {
		if self.end == self.ptr {
			return None;
		}

		let ret = unsafe {
			CStr::from_ptr(self.ptr)
				.to_str()
				.expect("Invalid .strtab contents")
		};

		self.ptr = unsafe { self.ptr.add(ret.len() + 1) };

		return Some(ret);
	}
}

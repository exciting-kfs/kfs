use core::{ffi::{c_char, CStr}, marker::PhantomData};

use rustc_demangle::demangle;

/// String table in the '.strtab' section, used to get a symbol name.
pub struct Strtab {
	addr: *const u8,
	size: usize
}

impl Strtab {
	pub const fn new() -> Self {
		Strtab { addr: 0 as *const u8, size: 0 }
	}

	pub fn init(&mut self, addr: *const u8, size: usize) {
		*self = Strtab { addr, size }
	}

	/// Get the name formed C style string and transform to a string slice.
	pub fn get_name(&self, index: isize) -> Option<&'static str> {
		let start = unsafe { self.addr.offset(index) } as *const c_char;
		unsafe { CStr::from_ptr(start).to_str().ok() }
	}

	pub fn get_name_index(&self, s: &str) -> Option<usize> {
		// TODO contains ? eq
		self.iter().find(|name| demangle(name).as_str().contains(s)).map(|name| {
			let table = self.addr as usize;
			let string = name.as_ptr() as usize;
			string - table
		})
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
	ptr: *const u8,
	end: *const u8,
	p: PhantomData<&'a u8>
}

impl<'a> Iter<'a> {
	fn new(cont: &Strtab) -> Self {
		Iter {
			ptr: cont.addr,
			end: unsafe { cont.addr.add(cont.size) },
			p: PhantomData
		}
	}
}

impl<'a> Iterator for Iter<'a> {
	type Item = &'a str;
	fn next(&mut self) -> Option<Self::Item> {
		if self.end == self.ptr {
			return None;
		}

		let len = c_strlen(self.ptr);
		let ret = unsafe { core::slice::from_raw_parts(self.ptr, len) };
		let ret = core::str::from_utf8(ret).ok();
		self.ptr = unsafe { self.ptr.add(len + 1) };

		ret
	}
}

fn c_strlen(ptr: *const u8) -> usize {
	let mut len = 0;
	while unsafe { *ptr.add(len) } != 0 {
		len += 1
	}
	len
}

use core::ffi::CStr;

use super::ElfError;

pub struct StringTable<'a>(&'a [u8]);

impl<'a> StringTable<'a> {
	pub fn new(raw: &'a [u8]) -> Self {
		Self(raw)
	}

	pub fn lookup_by_idx(&self, idx: usize) -> Result<&str, ElfError> {
		if self.0.len() <= idx {
			return Err(ElfError::StringNotFound);
		}

		let c_str =
			CStr::from_bytes_until_nul(&self.0[idx..]).map_err(|_| ElfError::StringNotFound)?;

		c_str.to_str().map_err(|_| ElfError::StringNotFound)
	}
}

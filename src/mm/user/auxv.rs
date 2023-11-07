#[repr(usize)]
#[derive(Clone, Copy)]
pub enum AuxEntryType {
	Null = 0,
	Ignore = 1,
	Phdr = 3,
	Phent = 4,
	Phnum = 5,
	Pagesz = 6,
	Base = 7,
	Entry = 9,
	Execfn = 31,
}

#[repr(C)]
pub struct AuxEntry {
	kind: AuxEntryType,
	value: usize,
}

impl AuxEntry {
	pub fn new_null() -> Self {
		Self {
			kind: AuxEntryType::Null,
			value: 0,
		}
	}

	pub fn new(kind: AuxEntryType, value: usize) -> Self {
		Self { kind, value }
	}

	pub fn serialize(&self) -> [usize; 2] {
		[self.value, self.kind as usize]
	}
}

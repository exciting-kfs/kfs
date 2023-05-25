use super::{Strtab, Symtab};

#[derive(Clone)]
pub struct KernelSymbol {
	symtab: Symtab,
	strtab: Strtab,
}

impl KernelSymbol {
	pub fn new(symtab: Symtab, strtab: Strtab) -> Self {
		Self { symtab, strtab }
	}

	pub fn find_name_by_addr(&self, addr: *const usize) -> Option<&'static str> {
		self.symtab
			.get_name_index(addr)
			.and_then(|idx| self.strtab.get_name(idx))
	}
}

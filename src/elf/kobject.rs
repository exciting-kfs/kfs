mod load_section;
mod module;
mod rel_section;

pub use module::KernelModule;

use alloc::collections::BTreeMap;

use crate::elf::relocation::RelocationType;
use crate::elf::{Relocation, SectionHdrNdx, SectionType};

use crate::ptr::VirtPageBox;

use self::load_section::{parse_load_sections, LoadSections};

use super::{Elf, ElfError, Symbol};

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct SectionIdx(usize);

pub struct KernelObject<'a> {
	load_base: VirtPageBox,
	load_sections: LoadSections<'a>,
	offset_map: BTreeMap<SectionIdx, usize>,
}

impl<'a> KernelObject<'a> {
	pub fn new(elf: &'a Elf<'a>) -> Result<Self, ElfError> {
		let load = parse_load_sections(elf)?;

		let mut size = 0;
		let mut offset_map = BTreeMap::new();

		for record in load.iter_sections() {
			size = record.offset + record.section.sh_size as usize;
			offset_map.insert(record.idx, record.offset);
		}

		let load_base = VirtPageBox::new(size).map_err(|_| ElfError::OutOfMemory)?;

		Ok(Self {
			load_base,
			load_sections: load,
			offset_map,
		})
	}

	fn copy_sections(&mut self) {
		for record in self.load_sections.iter_sections() {
			let dst = &mut self.load_base.as_mut_slice()
				[record.offset..(record.offset + record.section.sh_size as usize)];

			if matches!(record.section.get_type(), Ok(SectionType::Nobits)) {
				dst.fill(0);
			} else {
				dst.copy_from_slice(
					&self.load_sections.elf.raw[record.section.sh_offset
						..(record.section.sh_offset + record.section.sh_size as usize)],
				);
			}
		}
	}

	fn get_section_address_by_idx(&self, idx: SectionIdx) -> Result<usize, ElfError> {
		self.offset_map
			.get(&idx)
			.ok_or(ElfError::SectionNotFound)
			.map(|x| *x + self.load_base.as_ptr() as usize)
	}

	fn get_symbol_address(&self, symbol: &Symbol) -> Result<usize, ElfError> {
		use SectionHdrNdx::*;
		match symbol.get_related_section() {
			Normal(x) => {
				Ok(self.get_section_address_by_idx(SectionIdx(x as usize))? + symbol.st_value)
			}
			Absolute => Ok(symbol.st_value),
			UnknownReserved => Err(ElfError::InvalidRelatedSection),
		}
	}

	fn get_symbol_address_by_idx(&self, sym_idx: usize) -> Result<usize, ElfError> {
		let symbol = &self.load_sections.elf.symbol_table[sym_idx];

		self.get_symbol_address(symbol)
	}

	fn get_symbol_address_by_name(&self, name: &str) -> Result<usize, ElfError> {
		let strtab = &self.load_sections.elf.string_table;

		let sym_idx = self
			.load_sections
			.elf
			.symbol_table
			.iter()
			.position(|x| {
				strtab
					.lookup_by_idx(x.st_name as usize)
					.is_ok_and(|x| x == name)
			})
			.ok_or(ElfError::SymbolNotFound)?;

		self.get_symbol_address_by_idx(sym_idx)
	}

	fn resolve_relocation(rel: &Relocation, sym_address: usize, dst: *mut usize) {
		use RelocationType::*;
		match rel.get_type() {
			R_386_32 => unsafe { dst.write_unaligned(sym_address) },
			R_386_PC32 | R_386_PLT32 => unsafe {
				dst.write_unaligned(
					sym_address
						.wrapping_sub(dst as usize)
						.wrapping_add(dst.read_unaligned()),
				)
			},
			UNKNOWN => panic!("unknown relocation model."),
		}
	}

	pub fn load(mut self) -> Result<KernelModule, ElfError> {
		self.copy_sections();
		for rel_section in &self.load_sections.rel {
			for rel in rel_section.rel {
				let dst = (self.get_section_address_by_idx(rel_section.target)? + rel.get_offset())
					as *mut usize;

				let sym_address = self.get_symbol_address_by_idx(rel.get_symbol_index())?;

				Self::resolve_relocation(rel, sym_address, dst);
			}
		}

		let init_address = self.get_symbol_address_by_name("init_module")?;

		Ok(KernelModule::new(self.load_base, init_address))
	}
}

use core::mem::size_of;

use crate::elf::{check_size_and_deref_array, Elf, ElfError, Relocation, SectionHdr, SectionType};

use super::SectionIdx;

pub struct RelocationSection<'a> {
	pub target: SectionIdx,
	pub rel: &'a [Relocation],
}

impl<'a> RelocationSection<'a> {
	pub fn new(elf: &'a Elf<'a>, section: &'a SectionHdr) -> Result<Self, ElfError> {
		let Ok(SectionType::Rel) = section.get_type() else {
			return Err(ElfError::InvalidSectionType);
		};

		Ok(Self {
			target: SectionIdx(section.sh_info as usize),
			rel: check_size_and_deref_array::<Relocation>(
				elf.raw,
				section.sh_offset,
				section.sh_size as usize / size_of::<Relocation>(),
			)?,
		})
	}
}

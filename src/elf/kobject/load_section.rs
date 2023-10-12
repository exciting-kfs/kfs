use alloc::vec::Vec;

use crate::elf::{Elf, ElfError, SectionFlag, SectionHdr, SectionType};
use crate::mm::{constant::PAGE_SIZE, util::next_align};

use super::{rel_section::RelocationSection, SectionIdx};

pub struct LoadSections<'a> {
	pub elf: &'a Elf<'a>,
	pub rel: Vec<RelocationSection<'a>>,
	text: Vec<SectionIdx>,
	data: Vec<SectionIdx>,
	bss: Vec<SectionIdx>,
}

impl<'a> LoadSections<'a> {
	pub fn get_section_idx_by_cursor(&self, cursor: LoadSectionCursor) -> Option<SectionIdx> {
		use LoadSectionCursor::*;
		match cursor {
			Text(x) => Some(self.text[x]),
			Data(x) => Some(self.data[x]),
			Bss(x) => Some(self.bss[x]),
			End => None,
		}
	}

	pub fn iter_sections(&'a self) -> LoadSectionIter<'a> {
		LoadSectionIter::new(self)
	}
}

#[derive(Clone, Copy)]
pub enum LoadSectionCursor {
	Text(usize),
	Data(usize),
	Bss(usize),
	End,
}

impl LoadSectionCursor {
	fn next_idx(self) -> Self {
		use LoadSectionCursor::*;
		match self {
			Text(x) => Text(x + 1),
			Data(x) => Data(x + 1),
			Bss(x) => Bss(x + 1),
			End => End,
		}
	}

	fn next_section(self) -> Self {
		use LoadSectionCursor::*;
		match self {
			Text(_) => Data(0),
			Data(_) => Bss(0),
			Bss(_) | End => End,
		}
	}
}
pub struct LoadSectionIter<'a> {
	cursor: LoadSectionCursor,
	offset: usize,
	sections: &'a LoadSections<'a>,
}

pub struct LoadSectionRecord<'a> {
	pub idx: SectionIdx,
	pub section: &'a SectionHdr,
	pub offset: usize,
}

impl<'a> LoadSectionIter<'a> {
	pub fn new(sections: &'a LoadSections<'a>) -> Self {
		use LoadSectionCursor::*;
		let cursor = if !sections.text.is_empty() {
			Text(0)
		} else if !sections.data.is_empty() {
			Data(0)
		} else if !sections.bss.is_empty() {
			Bss(0)
		} else {
			End
		};

		Self {
			cursor,
			offset: 0,
			sections,
		}
	}

	fn is_valid_cursor(&self, cursor: &LoadSectionCursor) -> bool {
		use LoadSectionCursor::*;
		match cursor {
			Text(x) => *x < self.sections.text.len(),
			Data(x) => *x < self.sections.data.len(),
			Bss(x) => *x < self.sections.bss.len(),
			End => true,
		}
	}

	fn get_current_section(&self) -> Option<&'a SectionHdr> {
		self.sections
			.get_section_idx_by_cursor(self.cursor)
			.map(|x| &self.sections.elf.section_hdrs[x.0])
	}

	fn advance(&mut self) {
		if let Some(section) = self.get_current_section() {
			self.offset += section.sh_size as usize;
		}

		self.cursor = self.cursor.next_idx();
		while !self.is_valid_cursor(&self.cursor) {
			self.cursor = self.cursor.next_section();
			self.offset = next_align(self.offset, PAGE_SIZE);
		}

		if let Some(section) = self.get_current_section() {
			self.offset = next_align(self.offset, section.sh_addralign as usize);
		}
	}
}

impl<'a> Iterator for LoadSectionIter<'a> {
	type Item = LoadSectionRecord<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		let ret = self
			.sections
			.get_section_idx_by_cursor(self.cursor)
			.map(|idx| {
				let section = &self.sections.elf.section_hdrs[idx.0];

				LoadSectionRecord {
					idx,
					section,
					offset: self.offset,
				}
			});

		self.advance();

		ret
	}
}

pub fn parse_load_sections<'a>(elf: &'a Elf<'a>) -> Result<LoadSections<'a>, ElfError> {
	let mut text = Vec::new();
	let mut data = Vec::new();
	let mut bss = Vec::new();
	let mut rel = Vec::new();

	for (idx, section) in elf.section_hdrs.iter().enumerate() {
		if matches!(section.get_type(), Ok(SectionType::Rel)) {
			rel.push(RelocationSection::new(elf, section)?);
		} else if section.has_flag(SectionFlag::ALLOC) {
			if matches!(section.get_type(), Ok(SectionType::Nobits)) {
				bss.push(SectionIdx(idx));
			} else if section.has_flag(SectionFlag::WRITE) {
				data.push(SectionIdx(idx));
			} else {
				text.push(SectionIdx(idx));
			}
		}
	}

	Ok(LoadSections {
		elf,
		text,
		data,
		bss,
		rel,
	})
}

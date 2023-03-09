//! Backtrace

mod register;
mod stack_dump;
mod stackframe;
mod stackframe_iter;
mod strtab;
mod symtab;

use multiboot2::BootInformation;
use multiboot2::ElfSection;

use strtab::Strtab;
use symtab::Symtab;

use crate::backtrace::symtab::SymtabEntry;
use crate::pr_info;
use crate::BOOT_INFO;

pub use stack_dump::StackDump;

pub struct Backtrace {
	stack: StackDump,
	tables: Option<(Symtab, Strtab)>,
}

impl Backtrace {
	pub fn new(stack: StackDump) -> Self {
		let boot_info = unsafe { multiboot2::load(BOOT_INFO).ok() };

		let tables = boot_info.and_then(get_sections).and_then(make_tables);

		Backtrace { stack, tables }
	}

	/// Print call stack trace of StackDump.
	pub fn print_trace(&self) {
		for (idx, frame) in self.stack.iter().enumerate() {
			let name = self.find_name(frame.fn_addr);
			pr_info!("frame #{}: {:?}: {:?}", idx, frame.fn_addr, name);
		}
	}

	/// Find function name using Symtab and Strtab
	fn find_name(&self, fn_addr: *const usize) -> &'static str {
		if let None = self.tables {
			return "";
		}
		let (symtab, strtab) = self.tables.as_ref().unwrap();
		let index = symtab.get_name_index(fn_addr);
		let name = strtab.get_name(index);
		name.unwrap_or_default()
	}
}

/// Make Symtab and Strtab using ELF sections.
fn make_tables(sections: (ElfSection, ElfSection)) -> Option<(Symtab, Strtab)> {
	let (symtab, strtab) = sections;
	let addr = symtab.start_address() as *const SymtabEntry;
	let count = symtab.size() as usize / core::mem::size_of::<SymtabEntry>();
	let symtab = Symtab::new(addr, count);
	let strtab = Strtab::new(strtab.start_address() as *const u8);

	Some((symtab, strtab))
}

/// Get elf sections named ".symtab" and ".strtab" in multiboot2 boot information.
fn get_sections(boot_info: BootInformation) -> Option<(ElfSection, ElfSection)> {
	let elf_section_tag = boot_info.elf_sections_tag()?;
	let elf_section_iter = elf_section_tag.sections();

	let mut symtab = None;
	let mut strtab = None;
	for section in elf_section_iter {
		if section.name() == ".symtab" {
			symtab = Some(section);
		} else if section.name() == ".strtab" {
			strtab = Some(section);
		}
	}
	Some((symtab?, strtab?))
}

/// Print call stack trace in current context.
#[macro_export]
macro_rules! print_stacktrace {
	() => {
		let dump = $crate::backtrace::StackDump::new();
		let bt = $crate::backtrace::Backtrace::new(dump);
		bt.print_trace();
	};
}

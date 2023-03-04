//! Backtrace

mod stackframe;
mod stackframe_iter;
mod register;
mod stack_dump;
mod symtab;
mod strtab;

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
    symtab: Option<Symtab>,
    strtab: Option<Strtab>
}

impl Backtrace {
    pub fn new(stack: StackDump) -> Self {
        let boot_info = unsafe {
            let bi_addr = BOOT_INFO.expect("boot information: not existed");
            multiboot2::load(bi_addr as usize).expect("boot information: invalid address or format")
        };

        let (symtab, strtab) = get_sections(boot_info);
        let (symtab, strtab) = make_tables(symtab, strtab);
        Backtrace {
            stack,
            symtab,
            strtab
        }
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
        let index = self.symtab.as_ref()
            .map(|sym| sym.get_name_index(fn_addr)).flatten();
        let name = self.strtab.as_ref()
            .map(|str| str.get_name(index)).flatten();
        name.unwrap_or_default()
    }
}

/// Make Symtab and Strtab using ELF sections.
fn make_tables(symtab: Option<ElfSection>, strtab: Option<ElfSection>) -> (Option<Symtab>, Option<Strtab>) {
    let symtab = symtab.map(|section| {
        let addr = section.start_address() as *const SymtabEntry;
        let count =  section.size() as usize / core::mem::size_of::<SymtabEntry>();
        Symtab::new(addr, count)
    });
    let strtab = strtab.map(|section| {
        Strtab::new(section.start_address() as *const u8)
    });

    (symtab, strtab)
}

/// Get elf sections named ".symtab" and ".strtab" in multiboot2 boot information.
fn get_sections(boot_info: BootInformation) -> (Option<ElfSection>, Option<ElfSection>) {

    let elf_section_tag = boot_info.elf_sections_tag().unwrap();
	let elf_section_iter = elf_section_tag.sections();
    
    let mut symtab = None;
    let mut strtab = None;
	for section in elf_section_iter {
		if section.name() == ".symtab" {
            symtab = Some(section);
		}
		else if section.name() == ".strtab" {
            strtab = Some(section);
		}
	}
    (symtab, strtab)
}

/// Print call stack trace in current context.
#[macro_export]
macro_rules! print_stacktrace {
    () => {
        let dump = StackDump::new();
        let bt = Backtrace::new(dump);
        bt.print_trace();   
    };
}
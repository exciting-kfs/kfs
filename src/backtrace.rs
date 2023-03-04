mod stackframe;
mod stackframe_iter;
mod register;
mod stack_dump;
mod symtab;

pub use stack_dump::StackDump;

use symtab::Symtab;

use crate::pr_info;
use crate::BOOT_INFO;

pub struct Backtrace {
    stack: StackDump,
    symtab: *const Symtab,
    strtab: *const u8
}

impl Backtrace {
    pub fn new(stack: StackDump) -> Self {
        let (symtab, strtab) = get_tables();
        Backtrace {
            stack,
            symtab,
            strtab
        }
    }

    pub fn print_trace(self) {
        for frame in self.stack {
            pr_info!("{:?}: {:?}", frame.base_ptr, frame.fn_addr);
        }
    }
}

fn get_tables() -> (*const Symtab, *const u8) {
    let boot_info = unsafe {
        let bi_addr = match BOOT_INFO {
            Some(bi_addr) => bi_addr,
            None => panic!("There is no boot information!")
        };
        multiboot2::load(bi_addr as usize).expect("invalid address or format")
    };

    let elf_section_tag = boot_info.elf_sections_tag().unwrap();
	let elf_section_iter = elf_section_tag.sections();
    let mut symtab = 0;
    let mut strtab = 0;
	for section in elf_section_iter {
		if section.name() == ".symtab" {
            pr_info!("SYMTAB: {:#x}", section.start_address());
            symtab = section.start_address();
		}
		else if section.name() == ".strtab" {
            pr_info!("STRTAB: {:#x}", section.start_address());
            strtab = section.start_address();
		}
	}
    (symtab as *const Symtab, strtab as *const u8)
}
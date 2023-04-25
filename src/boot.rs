mod strtab;
mod symtab;

use core::ffi::c_char;
use core::ops::Range;

use crate::mm::util::to_phys;
use multiboot2::ElfSection;

use self::{
	strtab::Strtab,
	symtab::{Symtab, SymtabEntry},
};

const MULTIBOOT2_MAGIC: u32 = 0x36d76289;

pub static mut BOOT_INFO: usize = 0;
pub static mut SYMTAB: Symtab = Symtab::new();
pub static mut STRTAB: Strtab = Strtab::new();

pub struct MemInfo {
	pub linear: Range<usize>,
	pub kernel_end: usize,
}

pub fn init_bootinfo(bi_header: usize, magic: u32) -> MemInfo {
	check_magic(magic);

	unsafe { BOOT_INFO = bi_header };

	let info = unsafe { multiboot2::load(bi_header).unwrap() };
	let mmap = info.memory_map_tag().unwrap();

	let linear = mmap
		.memory_areas()
		.find(|x| x.start_address() == (1024 * 1024))
		.unwrap();

	let linear = (linear.start_address() as usize)..(linear.end_address() as usize);

	let kernel_end = info
		.elf_sections_tag()
		.unwrap()
		.sections()
		.fold(to_phys(bi_header + info.total_size()), |acc, cur| {
			acc.max(to_phys(cur.end_address() as usize))
		});

	return MemInfo { linear, kernel_end };
}

fn check_magic(magic: u32) {
	if magic != MULTIBOOT2_MAGIC {
		panic!(
			concat!(
				"unexpected boot magic. ",
				"expected: {:#x}, ",
				"but received: {:#x}",
			),
			MULTIBOOT2_MAGIC, magic
		);
	}
}

fn get_info(bi_header: usize) -> (ElfSection, ElfSection, usize) {
	let info = unsafe { multiboot2::load(bi_header).unwrap() };

	let mut symtab = None;
	let mut strtab = None;
	let mut last_address = unsafe { bi_header + *(bi_header as *const u32) as usize };

	let sh = info.elf_sections_tag().unwrap();
	for section in sh.sections() {
		last_address = last_address.max(section.end_address() as usize);
		if section.name() == ".symtab" {
			symtab = Some(section);
		} else if section.name() == ".strtab" {
			strtab = Some(section);
		}
	}

	let symtab = symtab.expect("There is no symtab.");
	let strtab = strtab.expect("There is no strtab.");

	(symtab, strtab, last_address)
}

fn set_tables(symtab: ElfSection, strtab: ElfSection) {
	unsafe {
		let addr = symtab.start_address() as *const SymtabEntry;
		let count = symtab.size() as usize / core::mem::size_of::<SymtabEntry>();
		SYMTAB.init(addr, count);

		let addr = strtab.start_address() as *const c_char;
		let size = strtab.size() as usize;
		STRTAB.init(addr, size);
	}
}

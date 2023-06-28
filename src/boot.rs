mod kernel_symbol;
mod p_memory;
mod strtab;
mod symtab;

use kernel_symbol::KernelSymbol;
use strtab::Strtab;
use symtab::Symtab;

use crate::acpi::RSDT_PADDR;

use kernel_symbol::KSYMS;
pub use p_memory::BootAlloc;
pub use p_memory::MEM_INFO;

const MULTIBOOT2_MAGIC: u32 = 0x36d7_6289;

#[derive(Debug)]
pub enum Error {
	InSufficientMemory,
	WrongMagic,
	FailedToLoadHeader,
	MissingSection,
	MissingElfHeader,
	MissingMemoryMap,
	MissingLinearMemory,
	MissingRSDP,
}

fn check_magic(magic: u32) -> bool {
	magic == MULTIBOOT2_MAGIC
}

pub fn init(bi_header: usize, magic: u32) -> Result<BootAlloc, Error> {
	if !check_magic(magic) {
		return Err(Error::WrongMagic);
	}

	let bi = unsafe { multiboot2::load(bi_header) }.map_err(|_| Error::FailedToLoadHeader)?;
	let mut kernel_end = 0;

	kernel_symbol::init(&bi, &mut kernel_end)?;
	p_memory::init(&bi, kernel_end)?;

	unsafe { RSDT_PADDR = bi.rsdp_v1_tag().ok_or(Error::MissingRSDP)?.rsdt_address() };

	Ok(BootAlloc::new())
}

pub fn get_ksyms() -> KernelSymbol {
	KSYMS.lock().clone()
}

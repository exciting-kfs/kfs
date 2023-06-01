mod kernel_symbol;
mod p_memory;
mod strtab;
mod symtab;

use core::ops::Range;

use kernel_symbol::KernelSymbol;
use strtab::Strtab;
use symtab::Symtab;

use crate::acpi::RSDT_PADDR;

use self::kernel_symbol::KSYMS;
use self::p_memory::MEM_INFO;

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
}

fn check_magic(magic: u32) -> bool {
	magic == MULTIBOOT2_MAGIC
}

pub fn init(bi_header: usize, magic: u32) -> Result<(), Error> {
	if !check_magic(magic) {
		return Err(Error::WrongMagic);
	}

	let bi = unsafe { multiboot2::load(bi_header) }.map_err(|_| Error::FailedToLoadHeader)?;
	let mut kernel_end = 0;

	kernel_symbol::init(&bi, &mut kernel_end)?;
	p_memory::init(&bi, kernel_end)?;

	unsafe { RSDT_PADDR = bi.rsdp_v1_tag().unwrap().rsdt_address() };

	Ok(())
}

pub unsafe fn allocate_n<T>(n: usize) -> *mut T {
	MEM_INFO.lock().alloc_n(n)
}

pub fn get_ksyms() -> KernelSymbol {
	KSYMS.lock().clone()
}

pub fn get_pmem_bound() -> Range<u64> {
	let pmem = MEM_INFO.lock();

	pmem.kernel_end..pmem.linear.end
}

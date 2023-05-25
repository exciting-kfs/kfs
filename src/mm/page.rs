mod arch;
mod os;

use crate::mm::util::*;
use core::alloc::AllocError;

pub use arch::{get_vmemory_map, PageFlag, VMemory};
pub(crate) use os::metapage_let;
pub use os::{
	alloc_meta_page_table, index_to_meta, meta_to_index, meta_to_ptr, ptr_to_meta, MetaPage,
};

pub fn to_phys(vaddr: usize) -> Option<usize> {
	arch::CURRENT_PD.lock().lookup(vaddr)
}

pub fn to_virt(paddr: usize) -> Option<usize> {
	let meta = os::index_to_meta(addr_to_pfn(paddr));

	unsafe { meta.as_ref().mapped_addr() }
}

pub fn map_page(vaddr: usize, paddr: usize, flags: PageFlag) -> Result<(), AllocError> {
	arch::CURRENT_PD.lock().map_page(vaddr, paddr, flags)?;

	let pfn = addr_to_pfn(paddr);
	unsafe { os::index_to_meta(pfn).as_mut().remap(vaddr) };

	Ok(())
}

pub fn unmap_page(vaddr: usize) -> Result<(), ()> {
	let mut pd = arch::CURRENT_PD.lock();

	let pfn = addr_to_pfn(pd.lookup(vaddr).ok_or_else(|| ())?);

	pd.unmap_page(vaddr)?;

	unsafe { os::index_to_meta(pfn).as_mut().remap(0) };

	Ok(())
}

pub unsafe fn init(ptr: *mut MetaPage, count: usize) {
	arch::init();
	os::init(ptr, count);
}

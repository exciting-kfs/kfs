mod arch;
mod os;

use crate::mm::util::*;
use core::{alloc::AllocError, ptr::NonNull};

pub use arch::{get_vmemory_map, mmio_init, PageFlag, VMemory};
pub(crate) use os::metapage_let;
pub use os::{
	alloc_meta_page_table, index_to_meta, meta_to_index, meta_to_ptr, ptr_to_meta, MetaPage,
};

use self::arch::{util::invalidate_all_tlb, PDE};

pub fn to_phys(vaddr: usize) -> Option<usize> {
	arch::CURRENT_PD.lock().lookup(vaddr)
}

pub fn to_virt(paddr: usize) -> Option<usize> {
	let meta = os::index_to_meta(addr_to_pfn(paddr));

	unsafe { meta.as_ref().mapped_addr() }
}

pub fn remap_page_4m(vaddr: usize, paddr: usize, flags: PageFlag) -> PageFlag {
	let mut pd = arch::CURRENT_PD.lock();
	let pd_idx = addr_to_pfn(vaddr);
	let pde = pd[pd_idx];

	pd.map_4m(vaddr, paddr, flags);

	invalidate_all_tlb(); // TODO invalidate one.
	pde.data()
}

pub fn restore_page_4m(vaddr: usize, backup: PageFlag) {
	let mut pd = arch::CURRENT_PD.lock();
	let pd_idx = addr_to_pfn(vaddr);

	pd[pd_idx] = PDE::from_data(backup);
	invalidate_all_tlb(); // TODO invalidate one.
}

fn map_page_table(vaddr: usize, paddr: usize, flags: PageFlag) -> Result<(), AllocError> {
	arch::CURRENT_PD.lock().map_page(vaddr, paddr, flags)
}

pub fn map_page(vaddr: usize, paddr: usize, flags: PageFlag) -> Result<(), AllocError> {
	map_page_table(vaddr, paddr, flags)?;

	let pfn = addr_to_pfn(paddr);
	unsafe { os::index_to_meta(pfn).as_mut().remap(vaddr) };

	Ok(())
}

pub fn map_mmio(vaddr: usize, paddr: usize, flags: PageFlag) -> Result<(), AllocError> {
	map_page_table(vaddr, paddr, flags)
}

pub fn unmap_page(vaddr: usize) -> Result<(), ()> {
	let mut pd = arch::CURRENT_PD.lock();

	let pfn = addr_to_pfn(pd.lookup(vaddr).ok_or_else(|| ())?); // PageNotFound
	unsafe { os::index_to_meta(pfn).as_mut().remap(0) };

	pd.unmap_page(vaddr)?; // InvalidAddress

	Ok(())
}

pub fn unmap_mmio(vaddr: usize) -> Result<(), ()> {
	arch::CURRENT_PD.lock().unmap_page(vaddr)
}

pub unsafe fn init(table: NonNull<[MetaPage]>) {
	arch::init();
	os::init(table);
}

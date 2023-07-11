mod arch;
mod os;

use core::{alloc::AllocError, ptr::NonNull};

pub use arch::{PageFlag, KERNEL_PD, PD, PDE, PT, PTE};
pub(crate) use os::metapage_let;
pub use os::{
	alloc_meta_page_table, index_to_meta, meta_to_index, meta_to_ptr, ptr_to_meta, MetaPage,
};

use super::util::invlpg;

pub fn to_phys(vaddr: usize) -> Option<usize> {
	arch::KERNEL_PD.lock().lookup(vaddr)
}

pub fn map_page(vaddr: usize, paddr: usize, flags: PageFlag) -> Result<(), AllocError> {
	arch::KERNEL_PD.lock().map_page(vaddr, paddr, flags)
}

pub fn unmap_page(vaddr: usize) -> Result<(), ()> {
	KERNEL_PD.lock().unmap_page(vaddr)?;
	invlpg(vaddr);

	Ok(())
}

pub unsafe fn init(table: NonNull<[MetaPage]>) {
	arch::init();
	os::init(table);
}

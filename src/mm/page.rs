mod arch;
mod os;

use core::ptr::NonNull;

pub use arch::{PageFlag, KERNEL_PD, PD, PDE, PT, PTE};
pub(crate) use os::metapage_let;
pub use os::{
	alloc_meta_page_table, index_to_meta, meta_to_index, meta_to_ptr, ptr_to_meta, MetaPage,
};

pub unsafe fn init(table: NonNull<[MetaPage]>) {
	arch::init();
	os::init(table);
}

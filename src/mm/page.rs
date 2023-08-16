mod arch;
mod os;

use core::ptr::NonNull;

pub use arch::*;
pub(crate) use os::metapage_let;
pub use os::{
	alloc_meta_page_table, index_to_meta, meta_to_index, meta_to_unmapped, phys_to_meta, MetaPage,
};

pub unsafe fn init_metapage_table(table: NonNull<[MetaPage]>) {
	os::init(table);
}

use core::mem::{size_of, MaybeUninit};
use core::ops::IndexMut;
use core::ptr::NonNull;
use core::slice::from_raw_parts_mut;

use crate::boot::{self, BootAlloc};
use crate::mm::util::*;
use crate::ptr::UnMapped;
use crate::sync::locked::Locked;

use super::metapage::MetaPage;

static META_PAGE_TABLE: Locked<MaybeUninit<&'static mut [MetaPage]>> = Locked::uninit();

pub unsafe fn alloc_meta_page_table(bootalloc: &mut BootAlloc) -> NonNull<[MetaPage]> {
	let page_count = unsafe { boot::MEM_INFO.end_pfn };

	bootalloc.alloc_n::<MetaPage>(page_count)
}

pub unsafe fn init(table: NonNull<[MetaPage]>) {
	let base_ptr = table.as_ptr() as *mut MetaPage;
	let count = table.len();

	for entry in (0..count).map(|x| base_ptr.add(x)) {
		MetaPage::construct_at(entry);
	}

	META_PAGE_TABLE
		.lock()
		.as_mut_ptr()
		.write(from_raw_parts_mut(base_ptr, count));
}

// TODO atomic?
pub fn meta_to_unmapped(page: NonNull<MetaPage>) -> UnMapped {
	let index = meta_to_index(page);

	unsafe { UnMapped::new(pfn_to_addr(index), page.as_ref().rank()) }
}

pub fn phys_to_meta(ptr: usize) -> NonNull<MetaPage> {
	index_to_meta(addr_to_pfn(ptr))
}

pub fn meta_to_index(page: NonNull<MetaPage>) -> usize {
	let addr = page.as_ptr() as usize;
	let base = unsafe { META_PAGE_TABLE.lock().assume_init_mut().as_ptr() } as usize;

	(addr - base) / size_of::<MetaPage>()
}

pub fn index_to_meta(index: usize) -> NonNull<MetaPage> {
	NonNull::from(unsafe { META_PAGE_TABLE.lock().assume_init_mut() }.index_mut(index))
}

use core::mem::{size_of, MaybeUninit};
use core::ops::IndexMut;
use core::ptr::NonNull;
use core::slice::from_raw_parts_mut;

use crate::boot::{self, BootAlloc};
use crate::mm::util::*;
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

pub fn meta_to_ptr(page: NonNull<MetaPage>) -> NonNull<u8> {
	let index = meta_to_index(page);

	return unsafe { NonNull::new_unchecked(phys_to_virt(pfn_to_addr(index)) as *mut u8) };
}

pub fn ptr_to_meta(ptr: NonNull<u8>) -> NonNull<MetaPage> {
	let index = addr_to_pfn(virt_to_phys(ptr.as_ptr() as usize));

	return index_to_meta(index);
}

pub fn meta_to_index(page: NonNull<MetaPage>) -> usize {
	let addr = page.as_ptr() as usize;
	let base = unsafe { META_PAGE_TABLE.lock().assume_init_mut().as_ptr() } as usize;

	(addr - base) / size_of::<MetaPage>()
}

pub fn index_to_meta(index: usize) -> NonNull<MetaPage> {
	NonNull::from(unsafe { META_PAGE_TABLE.lock().assume_init_mut() }.index_mut(index))
}

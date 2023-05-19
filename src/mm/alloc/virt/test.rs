use kfs_macro::ktest;

use super::*;

use crate::mm::constant::*;
use crate::util::lcg::LCG;
use crate::{pr_err, pr_warn};
use alloc::vec::Vec;

static mut PAGE_STATE: [bool; (usize::MAX >> PAGE_SHIFT) + 1] =
	[false; (usize::MAX >> PAGE_SHIFT) + 1];

const RANDOM_SEED: u32 = 42;
const ALLOC_QUEUE_SIZE: usize = 100;

#[derive(Clone, Copy)]
struct AllocInfo {
	pub ptr: NonNull<[u8]>,
	pub layout: Layout,
}

fn reset_page_state() {
	for x in unsafe { PAGE_STATE.iter_mut() } {
		*x = false;
	}
}

fn mark_alloced(addr: usize, size: usize) {
	let pfn = addr >> PAGE_SHIFT;

	for i in pfn..(pfn + (size / PAGE_SIZE)) {
		unsafe {
			if PAGE_STATE[i] {
				panic!("allocation overwrapped!");
			}
			PAGE_STATE[i] = true;
		}
	}
}

fn mark_freed(addr: usize, size: usize) {
	let pfn = addr >> PAGE_SHIFT;

	for i in pfn..(pfn + (size / PAGE_SIZE)) {
		unsafe {
			if !PAGE_STATE[i] {
				panic!("double free detected.");
			}
			PAGE_STATE[i] = false;
		}
	}
}

fn alloc(size: usize) -> Result<AllocInfo, ()> {
	let layout = Layout::from_size_align(size, PAGE_SIZE).unwrap();
	let mut mem = VMALLOC.allocate(layout).or_else(|_| Err(()))?;

	let mem_slice = unsafe { mem.as_mut() };

	assert!(mem_slice.len() >= size);

	let l = mem_slice.len();
	let r = vsize(NonNull::new(mem_slice.as_mut_ptr()).unwrap());
	assert!(l == r);

	let addr = mem_slice.as_ptr() as usize;

	assert!(addr % PAGE_SIZE == 0);
	mark_alloced(addr, mem.len());

	unsafe { core::ptr::write_bytes(addr as *mut u8, 0, size) };

	Ok(AllocInfo { ptr: mem, layout })
}

fn free(info: AllocInfo) {
	let mem = unsafe { info.ptr.as_ref() };
	let addr = mem.as_ptr() as usize;
	mark_freed(addr, mem.len());
	unsafe { VMALLOC.deallocate(NonNull::new_unchecked(addr as *mut u8), info.layout) };
}

#[ktest]
pub fn vmalloc_basic() {
	reset_page_state();

	let data = alloc(2 * PAGE_SIZE).unwrap();

	free(data);
}

#[ktest]
pub fn vmalloc_alloc_free() {
	reset_page_state();

	let mut list: Vec<AllocInfo> = Vec::new();
	let mut rng = LCG::new(42);

	while let Ok(data) = alloc(((rng.rand() % 31) as usize + 1) * MB) {
		list.push(data);
	}

	let len = list.len();
	for _ in 0..(len * 10) {
		let l = rng.rand() as usize % len;
		let r = rng.rand() as usize % len;
		list.swap(l, r);
	}

	while let Some(data) = list.pop() {
		free(data);
	}
}

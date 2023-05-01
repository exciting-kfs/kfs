use core::mem::{align_of, size_of};

use super::{util::next_align_64, x86::init::VMEMORY};

pub unsafe fn alloc_n<T>(n: usize) -> *mut T {
	let vm = VMEMORY.assume_init_mut();

	let begin = next_align_64(vm.reserved.end, align_of::<T>() as u64);
	let end = begin + size_of::<T>() as u64 * n as u64;
	let limit = vm.normal.end;

	assert!(end <= limit);

	vm.reserved.end = end;

	begin as *mut T
}

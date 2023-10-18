#![no_std]

#[no_link]
extern crate alloc;

#[no_link]
extern crate kernel;

use alloc::sync::Arc;
use kernel::{elf::kobject::KernelModule, kernel_module, syscall::errno::Errno};

kernel_module! {
	name: b"hello",
	init: init_module,
	cleanup: Some(cleanup_module),
}

pub fn init_module(_this: Arc<KernelModule>) -> Result<(), Errno> {
	let mut arr: [u8; 1024] = [0; 1024];

	arr[0] = 4;
	kernel::do_something();

	Ok(())
}

pub fn cleanup_module() {
	kernel::do_something();
}

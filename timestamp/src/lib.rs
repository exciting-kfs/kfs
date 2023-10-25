#![no_std]

mod handle;
mod inode;

use alloc::sync::Arc;
use inode::TimestampInode;
use kernel::{
	elf::kobject::KernelModule,
	fs::{devfs, vfs::VfsInode},
	kernel_module,
	syscall::errno::Errno,
};

#[no_link]
extern crate alloc;

#[no_link]
extern crate kernel;

const NAME: &[u8] = b"timestamp";

kernel_module! {
	name: NAME,
	init: init_module,
	cleanup: Some(cleanup_module),
}

fn init_module(module: Arc<KernelModule>) -> Result<(), Errno> {
	let device = VfsInode::File(Arc::new(TimestampInode::new(&module)));
	devfs::register_device(NAME, device)
}

fn cleanup_module() {
	devfs::unregister_device(NAME);
}

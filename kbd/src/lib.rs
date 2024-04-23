#![no_std]
#![allow(dead_code)]

use alloc::sync::Arc;
use kernel::{
	elf::kobject::KernelModule,
	interrupt::{irq_disable, irq_enable},
	kernel_module,
	syscall::errno::Errno,
};

#[no_link]
extern crate alloc;

#[no_link]
extern crate kernel;

mod irq;
mod ps2;

kernel_module! {
	name: b"kbd",
	init: init_module,
	cleanup: None,
}

fn init_module(module: Arc<KernelModule>) -> Result<(), Errno> {
	irq::init(module)?;

	irq_disable(); // TODO why dose the interrupt automatically enabled?

	ps2::init().map_err(|_| Errno::EBADF)?;

	irq_enable();

	Ok(())
}

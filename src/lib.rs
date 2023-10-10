#![no_std]
#![allow(dead_code)]
#![feature(allocator_api)]
#![feature(maybe_uninit_uninit_array)]
#![feature(const_maybe_uninit_uninit_array)]
#![feature(new_uninit)]
#![feature(asm_const)]
#![feature(variant_count)]
#![feature(extract_if)]
#![feature(maybe_uninit_as_bytes)]
#![feature(slice_as_chunks)]
#![feature(unboxed_closures)]
#![feature(iter_intersperse)]
#![feature(get_mut_unchecked)]
#![feature(trait_upcasting)]

extern crate alloc;

pub mod acpi;
pub mod boot;
pub mod collection;
pub mod config;
pub mod driver;
pub mod elf;
pub mod fs;
pub mod input;
pub mod interrupt;
pub mod io;
pub mod mm;
pub mod net;
pub mod printk;
pub mod process;
pub mod ptr;
pub mod scheduler;
pub mod smp;
pub mod sync;
pub mod syscall;
pub mod test;
pub mod user_bin;
pub mod util;
pub mod x86;

use core::panic::PanicInfo;
use core::sync::atomic::AtomicBool;
use test::exit_qemu_with;

use crate::interrupt::irq_disable;
pub static RUN_TIME: AtomicBool = AtomicBool::new(false);

/// very simple panic handler.
/// that just print panic infomation and fall into infinity loop.
///
/// we should make sure no more `panic!()` from here.
#[panic_handler]
fn panic_handler_impl(info: &PanicInfo) -> ! {
	irq_disable();
	printk_panic!("{}\ncall stack (most recent call first)\n", info);

	print_stacktrace!();

	if cfg!(ktest) {
		exit_qemu_with(1);
	}

	loop {}
}

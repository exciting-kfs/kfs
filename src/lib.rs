#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(allocator_api)]
#![feature(maybe_uninit_uninit_array)]
#![feature(const_maybe_uninit_uninit_array)]
#![feature(asm_const)]

extern crate alloc;

mod acpi;
mod backtrace;
mod boot;
mod collection;
mod config;
mod console;
mod driver;
mod input;
mod interrupt;
mod io;
mod mm;
mod printk;
mod process;
mod ptr;
mod scheduler;
mod smp;
mod subroutine;
mod sync;
mod test;
mod user_bin;
mod util;
mod x86;

use console::{CONSOLE_COUNTS, CONSOLE_MANAGER};
use core::{arch::asm, panic::PanicInfo};
use process::context::yield_now;
use process::task::{Task, TASK_QUEUE};
use scheduler::work::slow_worker;
use test::{exit_qemu_with, TEST_ARRAY};
use user_bin::INIT_CODE;

/// very simple panic handler.
/// that just print panic infomation and fall into infinity loop.
///
/// we should make sure no more `panic!()` from here.
#[panic_handler]
fn panic_handler_impl(info: &PanicInfo) -> ! {
	printk_panic!("{}\ncall stack (most recent call first)\n", info);

	unsafe {
		print_stacktrace!();
		CONSOLE_MANAGER.get().set_foreground(CONSOLE_COUNTS - 1);
		CONSOLE_MANAGER.get().flush_foreground();
		CONSOLE_MANAGER.get().draw();
	};

	if cfg!(ktest) {
		exit_qemu_with(1);
	}

	loop {}
}

fn run_test() -> ! {
	let tests = TEST_ARRAY.as_slice();
	let n_test = tests.len();

	for (i, test) in tests.iter().enumerate() {
		pr_info!("Test [{}/{}]: {}", i + 1, n_test, test.get_name());
		test.run();
		pr_info!("...\x1b[32mPASS\x1b[39m");
	}

	pr_info!("All test PASSED.");

	exit_qemu_with(0);
}

fn idle() -> ! {
	loop {
		yield_now();
	}
}

fn run_process() -> ! {
	let a = Task::new_kernel(repeat_x as usize, 1111).expect("OOM");
	let b = Task::new_kernel(repeat_x as usize, 2222).expect("OOM");
	let c = Task::new_kernel(repeat_x as usize, 3333).expect("OOM");
	let worker = Task::new_kernel(slow_worker as usize, 0).expect("OOM");
	let init = Task::new_user(INIT_CODE).expect("OOM");

	TASK_QUEUE.lock().push_back(a);
	TASK_QUEUE.lock().push_back(b);
	TASK_QUEUE.lock().push_back(c);
	TASK_QUEUE.lock().push_back(worker);
	TASK_QUEUE.lock().push_back(init);

	idle();
}

extern "C" fn repeat_x(x: usize) -> ! {
	loop {
		pr_info!("FROM X={}", x);
		unsafe { asm!("hlt") }
	}
}

unsafe fn kernel_boot_alloc(bi_header: usize, magic: u32) {
	let mut bootalloc = boot::init(bi_header, magic).expect("boot information");

	let meta_page_table = mm::page::alloc_meta_page_table(&mut bootalloc);
	mm::page::init(meta_page_table);

	bootalloc.deinit();
}

#[no_mangle]
pub fn kernel_entry(bi_header: usize, magic: u32) -> ! {
	driver::vga::text_vga::init();
	driver::serial::init();

	// caution: order sensitive.
	unsafe {
		kernel_boot_alloc(bi_header, magic);
	}

	interrupt::apic::local::init().unwrap();
	interrupt::idt::init();

	mm::alloc::page::init();
	mm::alloc::phys::init();
	mm::alloc::virt::init();

	acpi::init();
	interrupt::apic::io::init().unwrap();

	unsafe { x86::init() };

	driver::ps2::init().expect("failed to init PS/2");
	scheduler::work::init().expect("worker thread init");

	process::init();

	match cfg!(ktest) {
		true => run_test(),
		false => run_process(),
	};
}

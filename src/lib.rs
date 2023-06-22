#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(allocator_api)]
#![feature(maybe_uninit_uninit_array)]
#![feature(const_maybe_uninit_uninit_array)]
#![feature(const_maybe_uninit_zeroed)]

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
mod smp;
mod subroutine;
mod sync;
mod test;
mod util;

use core::{arch::asm, panic::PanicInfo};

use alloc::collections::LinkedList;
use console::{CONSOLE_COUNTS, CONSOLE_MANAGER};
use driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar, Color};
use input::{key_event::Code, keyboard::KEYBOARD};
use process::{
	kthread::{kthread_create, kthread_exec},
	task::{CURRENT, TASK_QUEUE},
};
use test::{exit_qemu_with, TEST_ARRAY};

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

fn run_io() -> ! {
	let cyan = VGAChar::styled(VGAAttr::new(false, Color::Cyan, false, Color::Cyan), b' ');
	let magenta = VGAChar::styled(
		VGAAttr::new(false, Color::Magenta, false, Color::Magenta),
		b' ',
	);

	loop {
		if let Some(event) = unsafe { KEYBOARD.get_keyboard_event() } {
			if event.key == Code::Backtick && event.pressed() {
				pr_warn!("BACKTICK PRESSED!!");
				panic!("panic!!");
			}
			text_vga::putc(24, 79, cyan);
			unsafe {
				CONSOLE_MANAGER.get().update(event);
				CONSOLE_MANAGER.get().draw();
			};
		} else {
			unsafe {
				CONSOLE_MANAGER.get().flush_all();
			}
		}
		text_vga::putc(24, 79, magenta);
	}
}

fn run_process() -> ! {
	let a = kthread_create(repeat_x as usize, 1111).expect("OOM");
	let b = kthread_create(repeat_x as usize, 2222).expect("OOM");
	let c = kthread_create(repeat_x as usize, 3333).expect("OOM");

	unsafe { TASK_QUEUE.write(LinkedList::new()) };
	TASK_QUEUE.lock().push_back(b);
	TASK_QUEUE.lock().push_back(c);

	CURRENT.init(a);

	unsafe { kthread_exec(*CURRENT.get_mut().esp_mut()) };
}

extern "C" fn repeat_x(x: usize) -> ! {
	loop {
		pr_info!("FROM X={}", x);
		unsafe { asm!("hlt") }
	}
}

#[no_mangle]
pub fn kernel_entry(bi_header: usize, magic: u32) -> ! {
	driver::vga::text_vga::init();
	driver::serial::init();

	// caution: order sensitive.
	unsafe {
		boot::init(bi_header, magic).expect("boot information");
		let meta_page_table = mm::page::alloc_meta_page_table();
		mm::page::init(meta_page_table);

		mm::alloc::page::init();
		mm::alloc::phys::init();
		mm::alloc::virt::init();
		interrupt::idt::init();

		// after enabling collections.
		acpi::init();
		mm::page::mmio_init();
	}

	interrupt::apic::init();

	driver::ps2::init().expect("failed to init PS/2");

	// TODO keyboard interrupt handling.
	// unsafe { core::arch::asm!("sti") };

	match cfg!(ktest) {
		true => run_test(),
		false => run_process(),
	};
	// run_io();
}

#![no_std]
#![no_main]
#![allow(dead_code)]

mod backtrace;
mod boot;
mod collection;
mod console;
mod driver;
mod input;
mod io;
mod mm;
mod printk;
mod subroutine;

mod test;
mod util;

use core::panic::PanicInfo;

use console::{CONSOLE_COUNTS, CONSOLE_MANAGER};
use driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar, Color};
use input::{key_event::Code, keyboard::KEYBOARD};

use test::TEST_ARRAY;

/// very simple panic handler.
/// that just print panic infomation and fall into infinity loop.
///
/// we should make sure no more `panic!()` from here.
#[panic_handler]
fn panic_handler_impl(info: &PanicInfo) -> ! {
	printk_panic!("{}\ncall stack (most recent call first)\n", info);

	unsafe {
		if boot::BOOT_INFO != 0 {
			print_stacktrace!();
		}
		CONSOLE_MANAGER.get().set_foreground(CONSOLE_COUNTS - 1);
		CONSOLE_MANAGER.get().flush_foreground();
		CONSOLE_MANAGER.get().draw();
	};

	// Report panic to supervisor (qemu - pvpanic)
	if cfg!(ktest) {
		io::pmio::Port::new(0x505).write_byte(1);
	}

	loop {}
}

fn init_hardware() {
	text_vga::init_vga();
	driver::ps2::init_ps2().expect("failed to init PS/2");
	driver::serial::init_serial();
}

fn run_test() -> ! {
	for test in TEST_ARRAY.as_slice() {
		test.run();
	}

	loop {}
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
				static mut I: usize = 0;
				unsafe {
					pr_warn!("BACKTICK PRESSED {} TIMES!!", I);
					I += 1;
					panic!("panic!!");
				}
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

#[no_mangle]
pub fn kernel_entry(bi_header: usize, magic: u32) -> ! {
	init_hardware();

	let _kernel_end = boot::init_bootinfo(bi_header, magic);

	match cfg!(ktest) {
		true => run_test(),
		false => run_io(),
	};
}

mod test1111 {
	use super::*;
	use kfs_macro::ktest;

	#[ktest]
	pub fn do_something0() {
		pr_info!("DS: 0");
	}
	#[ktest]
	pub fn do_something1() {
		pr_info!("DS: 1");
	}
	#[ktest]
	pub fn do_something2() {
		pr_info!("DS: 2");
	}
	#[ktest]
	pub fn do_something3() {
		pr_info!("DS: 3");
	}
	#[ktest]
	pub fn do_something4() {
		pr_info!("DS: 4");
	}
}

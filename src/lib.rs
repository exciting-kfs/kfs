#![no_std]
#![no_main]
#![allow(dead_code)]

mod backtrace;
mod collection;
mod console;
mod driver;
mod input;
mod io;
mod mm;
mod printk;
mod subroutine;
mod util;
mod boot;

use core::panic::PanicInfo;

use console::{CONSOLE_COUNTS, CONSOLE_MANAGER};
use driver::{vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar, Color}, serial::{self, COM2}};
use input::{key_event::Code, keyboard::KEYBOARD};
use io::character::Write;
use subroutine::SHELL;

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

	loop {}
}

fn init_hardware() {
	text_vga::init_vga();
	driver::ps2::init_ps2().expect("failed to init PS/2");
	driver::serial::init_serial();
}


#[inline]
fn current_or_next_aligned(p: usize, align: usize) -> usize {
	(p + align - 1) & !(align - 1)
}

#[inline]
fn next_aligned(p: usize, align: usize) -> usize {
	(p + align) & !(align - 1)
}

fn run_test() {
	unsafe {
		COM2.wait_readable();
		while let Some(byte) = serial::COM2.get_byte() {
			SHELL.write_one(byte);
			CONSOLE_MANAGER.get().flush_all();
			CONSOLE_MANAGER.get().draw();
		}
	}
}

#[no_mangle]
pub fn kernel_entry(bi_header: usize, magic: u32) -> ! {
	init_hardware();

	let _kernel_end = boot::init_bootinfo(bi_header, magic);

	let cyan = VGAChar::styled(VGAAttr::new(false, Color::Cyan, false, Color::Cyan), b' ');
	let magenta = VGAChar::styled(
		VGAAttr::new(false, Color::Magenta, false, Color::Magenta),
		b' ',
	);

	run_test();

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

mod tests {
	use crate::pr_info;
	use kfs_macro::kernel_test;

	#[kernel_test]
	pub fn hello_world() {
		pr_info!("This function expanded to 'kernel_test_hello_world'");
	}

	#[kernel_test(example)]
	pub fn hello_world() {
		pr_info!("This function expanded to 'kernel_test_example_hello_world'");
	}

	#[no_mangle]
	pub fn hello_world() {
		pr_info!("unit_test NEVER run this function.");
	}
}
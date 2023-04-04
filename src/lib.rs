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
use driver::{vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar, Color}, serial};
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

#[no_mangle]
pub fn kernel_entry(bi_header: usize, magic: u32) -> ! {
	init_hardware();

	let _kernel_end = boot::init_bootinfo(bi_header, magic);

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
		} else if let Some(byte) = unsafe { serial::COM2.get_byte() } {
			unsafe {
				SHELL.write_one(byte);
				CONSOLE_MANAGER.get().flush_all();
				CONSOLE_MANAGER.get().draw();
			}
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

	#[allow(dead_code)]
	pub fn hello_world() {
		pr_info!("pr_info: hello world0");
	}

	#[no_mangle]
	pub fn hello_world_asdfghasdf() {
		pr_info!("pr_info: hello world1");
	}

	#[no_mangle]
	pub fn hello_world_asdfghasdfasdfasdfasdfas() {
		pr_info!("pr_info: hello world2");
	}
}

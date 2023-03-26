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

use core::panic::PanicInfo;

use console::{CONSOLE_COUNTS, CONSOLE_MANAGER};
use driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar, Color};
use input::{key_event::Code, keyboard::KEYBOARD};

pub static mut BOOT_INFO: usize = 0;

const MULTIBOOT2_MAGIC: u32 = 0x36d76289;

/// very simple panic handler.
/// that just print panic infomation and fall into infinity loop.
///
/// we should make sure no more `panic!()` from here.
#[panic_handler]
fn panic_handler_impl(info: &PanicInfo) -> ! {
	printk_panic!("{}\ncall stack (most recent call first)\n", info);

	unsafe {
		if BOOT_INFO != 0 {
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
	driver::serial::init_serial().expect("failed to init COM1 serial port");
}

fn init_bootinfo(bi_header: usize, magic: u32) -> usize {
	if magic != MULTIBOOT2_MAGIC {
		panic!(
			concat!(
				"unexpected boot magic. ",
				"expected: {:#x}, ",
				"but received: {:#x}",
			),
			MULTIBOOT2_MAGIC, magic
		);
	}

	unsafe { BOOT_INFO = bi_header };

	let mut last_address = unsafe { bi_header + *(bi_header as *const u32) as usize };

	let info = unsafe { multiboot2::load(bi_header).unwrap() };
	let sh = info.elf_sections_tag().unwrap();
	for section in sh.sections() {
		last_address = last_address.max(section.end_address() as usize);
	}

	last_address
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

	let _kernel_end = init_bootinfo(bi_header, magic);

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

#![no_std]
#![no_main]
#![allow(dead_code)]

mod collection;
mod console;
mod driver;
mod input;
mod io;
mod printk;
mod subroutine;
mod util;
mod backtrace;

use core::panic::PanicInfo;
use backtrace::Backtrace;

use console::{CONSOLE_COUNTS, CONSOLE_MANAGER};
use driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar, Color};
use input::{key_event::Code, keyboard::KEYBOARD};

/// very simple panic handler.
/// that just print panic infomation and fall into infinity loop.
///
/// we should make sure no more `panic!()` from here.
#[panic_handler]
fn panic_handler_impl(info: &PanicInfo) -> ! {
	unsafe { CONSOLE_MANAGER.get().set_foreground(CONSOLE_COUNTS - 1) };

	printk_panic!("{}", info);

	loop {}
}

#[no_mangle]
pub extern "C" fn kernel_entry(_boot_info: *const u32, _magic: u32) -> ! {
	let cyan = VGAChar::styled(VGAAttr::new(false, Color::Cyan, false, Color::Cyan), b' ');
	let magenta = VGAChar::styled(
		VGAAttr::new(false, Color::Magenta, false, Color::Magenta),
		b' ',
	);

	let b = Backtrace::new();

	// let boot_info = unsafe { multiboot2::load(_boot_info as usize).unwrap() };
	// let elf_section_tag = boot_info.elf_sections_tag().unwrap();
	// let elf_section_iter = elf_section_tag.sections();
	// for section in elf_section_iter {
	// 	if section.name() == ".symtab" {
	// 		pr_info!("{} : {:?} : {:#x} : {}", section.name(), section.section_type(), section.start_address(), section.size());
	// 	}
	// 	if section.name() == ".strtab" {
	// 		pr_info!("{} : {:?} : {:#x} : {}", section.name(), section.section_type(), section.start_address(), section.size());
	// 	}
	// 	if section.name() == ".shstrtab" {
	// 		pr_info!("{} : {:?} : {:#x} : {}", section.name(), section.section_type(), section.start_address(), section.size());
	// 	}
	// }

	text_vga::clear();
	text_vga::enable_cursor(0, 11);

	loop {
		if let Some(event) = unsafe { KEYBOARD.get_keyboard_event() } {
			if event.key == Code::Backtick && event.pressed() {
				static mut I: usize = 0;
				unsafe {
					pr_warn!("BACKTICK PRESSED {} TIMES!!", I);
					I += 1;
				}
			}
			text_vga::putc(24, 79, cyan);
			unsafe {
				CONSOLE_MANAGER.get().update(event);
				CONSOLE_MANAGER.get().draw();
			};
		}
		text_vga::putc(24, 79, magenta);
	}
}

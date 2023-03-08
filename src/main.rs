#![no_std]
#![no_main]
#![allow(dead_code)]

mod backtrace;
mod collection;
mod console;
mod driver;
mod input;
mod io;
mod multiboot2a;
mod printk;
mod subroutine;
mod util;

use core::{mem::size_of, panic::PanicInfo, slice::from_raw_parts};

use console::{CONSOLE_COUNTS, CONSOLE_MANAGER};
use driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar, Color};
use input::{key_event::Code, keyboard::KEYBOARD};
use multiboot2a::{BootInfo, BootInfoHeader, ElfSH, ElfSHTag, SymtabEntry};

/// very simple panic handler.
/// that just print panic infomation and fall into infinity loop.
///
/// we should make sure no more `panic!()` from here.
#[panic_handler]
fn panic_handler_impl(info: &PanicInfo) -> ! {
	unsafe { CONSOLE_MANAGER.get().set_foreground(CONSOLE_COUNTS - 1) };

	printk_panic!("{}\ncall stack (most recent call first)\n", info);
	print_stacktrace!();

	loop {}
}

pub static mut BOOT_INFO: usize = 0;

const MULTIBOOT2_MAGIC: u32 = 0x36d76289;

fn print_syms(sht: &ElfSHTag, sh: &ElfSH) {
	let addr = sh.sh_addr as usize as *const SymtabEntry;
	let len = sh.sh_size as usize / size_of::<SymtabEntry>();
	let entries = unsafe { from_raw_parts(addr, len) };

	let mut arr: [u32; 15] = [0; 15];
	for entry in entries.iter().rev() {
		let kind = entry.st_info & 0xf;

		arr[kind as usize] += 1;

		if (kind == 1) && entry.st_name != 0 {
			if let Some(name) = sht.lookup_name(326, entry.st_name as isize) {
				pr_info!("sym: {}, at: {:#x}", name, entry.st_value);
			}
		}
	}
	// for (i, v) in arr.iter().enumerate() {
	// 	pr_warn!("TYPE[{}]: {}", i, v);
	// }
}

#[no_mangle]
pub fn kernel_entry(bi_header: &'static BootInfoHeader, magic: u32) -> ! {
	text_vga::clear();
	text_vga::enable_cursor(0, 11);

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
	unsafe { BOOT_INFO = bi_header as *const _ as usize };

	let bi = BootInfo::load_from_header(bi_header);

	if let Some(sht) = bi.elf_sh() {
		for sh in sht.entries() {
			// pr_info!("{:?}", sht.section_name(sh));
			if sh.sh_type == 2 {
				print_syms(sht, sh);
			}
		}
	}

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

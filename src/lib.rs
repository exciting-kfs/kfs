#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(const_cmp)]
#![feature(allocator_api)]

extern crate alloc;

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

use core::{panic::PanicInfo, ptr::NonNull};
use core::arch::asm;

use boot::BOOT_INFO;
use console::{CONSOLE_COUNTS, CONSOLE_MANAGER};
use driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar, Color};
use input::{key_event::Code, keyboard::KEYBOARD};

use mm::{
	meta_page::MetaPageTable,
	x86::init::{VMemory, VMEMORY},
	PageAllocator,
};
use test::{exit_qemu_with, TEST_ARRAY};
use io::character::Write;
use kfs_macro::kernel_test;
use subroutine::SHELL;

use mm::page_allocator::buddy_allocator::{BuddyAllocator, Page};
use mm::x86_page::{PageFlag, PDE, PTE};

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

fn init_hardware() {
	text_vga::init_vga();
	driver::ps2::init_ps2().expect("failed to init PS/2");
	driver::serial::init_serial();
}

struct MemInfo {
	pub above_1m_start: usize,
	pub above_1m_end: usize,
	pub kernel_end: usize,
}

extern "C" {
	static mut GLOBAL_PD: mm::x86_page::PD;
	static mut GLOBAL_FIRST_PT: mm::x86_page::PT;
}

#[inline]
fn current_or_next_aligned(p: usize, align: usize) -> usize {
	(p + align - 1) & !(align - 1)
}

#[inline]
fn next_aligned(p: usize, align: usize) -> usize {
	(p + align) & !(align - 1)
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

fn init_pages(mem_info: &boot::MemInfo) {
	unsafe {
		GLOBAL_FIRST_PT.entries[0] = PTE::new(0, PageFlag::Present | PageFlag::Global);
		for i in 1..1024 {
			GLOBAL_FIRST_PT.entries[i] = PTE::new(
				0x1000 * i,
				PageFlag::Present | PageFlag::Global | PageFlag::Write,
			);
		}

		// 4M * 768 = 0xc000_0000
		GLOBAL_PD.entries[768] = PDE::new(
			&GLOBAL_FIRST_PT as *const _ as usize,
			PageFlag::Present | PageFlag::Write,
		);

		for i in 1..MAX_ZONE_NORMAL {
			if (i + 1) * PSE_PAGE_SIZE > mem_info.above_1m_end {
				break;
			}

			GLOBAL_PD.entries[i + 768] = PDE::new_4m(
				i * PSE_PAGE_SIZE,
				PageFlag::Present | PageFlag::Global | PageFlag::Write,
			);
		}

		flush_global_page_cache!();
	}

}

// 224 * 4 = 896MB
const MAX_ZONE_NORMAL: usize = 224;

// 4MB
const PSE_PAGE_SIZE: usize = 4 * 1024 * 1024;

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

	unsafe {
		boot::BootInfo::init(bi_header, magic).unwrap();
		VMemory::init(&BOOT_INFO.assume_init_ref().mem_info);
		MetaPageTable::init();
		PageAllocator::init(&VMEMORY.assume_init_ref());
	}

	match cfg!(ktest) {
		true => run_test(),
		false => run_io(),
	};
}

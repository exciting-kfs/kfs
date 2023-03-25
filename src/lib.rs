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

use test::{exit_qemu_with, TEST_ARRAY};

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
// -------------------------------------------------------------------

// #![allow(dead_code)]
// #![feature(maybe_uninit_uninit_array)]
// #![feature(new_uninit)]
// #![feature(const_maybe_uninit_uninit_array)]

// use crate::mm::page_allocator::buddy_allocator::Page;
// use crate::mm::page_allocator::util::rank_to_size;
// use core::mem::MaybeUninit;
// use rand::prelude::*;
// use std::ptr::NonNull;

// const SPACE_SIZE: usize = 1024 * 1024 * 1024; // (1GB)

// mod mm;

// use std::collections::BTreeMap;

// struct RangeMap {
//     min: usize,
//     max: usize,
//     tree: BTreeMap<usize, usize>,
// }

// impl RangeMap {
//     pub fn new(min: usize, max: usize) -> Self {
//         RangeMap {
//             min,
//             max,
//             tree: BTreeMap::new(),
//         }
//     }

//     pub fn add(&mut self, begin: usize, end: usize) -> Result<(), ()> {
//         assert!(begin < end);

//         if begin < self.min || self.max <= end {
//             println!(
//                 "b:{begin:#x} e:{end:#x}, min:{:#x}, max:{:#x}",
//                 self.min, self.max
//             );
//             return Err(());
//         }

//         // hmm...
//         if let Some(x) = self.tree.range(..begin).rev().next() {
//             if *x.1 > begin {
//                 println!("2");
//                 return Err(());
//             }
//         }

//         if let Some(x) = self.tree.range(begin..).next() {
//             if *x.0 < end {
//                 println!("3");
//                 return Err(());
//             }
//         }

//         self.tree.insert(begin, end);

//         return Ok(());
//     }
// }

// const PAGE_SIZE: usize = 4096;

// fn main() {
//     let space = Box::<MaybeUninit<[u8; SPACE_SIZE]>>::new_zeroed();
//     // MaybeUninit::uninit_array();
//     let mut page_alloc = unsafe {
//         mm::page_allocator::buddy_allocator::BuddyAllocator::new(
//             space.as_ptr() as usize,
//             space.as_ptr() as usize + SPACE_SIZE + 1,
//         )
//     };

//     let mut tree = RangeMap::new(
//         space.as_ptr() as usize,
//         space.as_ptr() as usize + SPACE_SIZE + 1,
//     );

//     let mut v: Vec<usize> = Vec::new();

//     let mut rng = rand::thread_rng();

//     let mut alloc_size = rng.gen_range(0..=10);
//     while let Ok(p) = page_alloc.alloc_page(alloc_size) {
//         let addr = p.as_ptr() as usize;
//         println!("ALLOC R={}", alloc_size);
//         v.push(addr);
//         tree.add(addr, addr + rank_to_size(alloc_size)).unwrap();
//         println!("{}", page_alloc);
//         alloc_size = rng.gen_range(0..=10);
//     }

//     for addr in v {
//         println!("FREE  {:p}", addr as *const u8);
//         page_alloc.free_page(NonNull::new(addr as *mut Page).unwrap());
//         println!("{}", page_alloc);
//     }
// }

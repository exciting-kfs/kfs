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

use core::{mem::MaybeUninit, panic::PanicInfo, ptr::NonNull};

use console::{CONSOLE_COUNTS, CONSOLE_MANAGER};
use driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar, Color};
use input::{key_event::Code, keyboard::KEYBOARD};

use mm::{
	x86::init::{init_linear_map, ZoneInfo},
	PageAllocator,
};
use test::{exit_qemu_with, TEST_ARRAY};

use crate::mm::util::phys_to_virt;

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

static mut PAGE_ALLOC: MaybeUninit<&'static mut PageAllocator> = MaybeUninit::uninit();
static mut ZONE_INFO: MaybeUninit<ZoneInfo> = MaybeUninit::uninit();

#[no_mangle]
pub fn kernel_entry(bi_header: usize, magic: u32) -> ! {
	init_hardware();

	let mem_info = boot::init_bootinfo(bi_header, magic);
	pr_info!(
		"meminfo: KERN_END: {:#0x}({:#0x}) LINEAR_S: {:#0x} LINEAR_E: {:#0x}",
		mem_info.kernel_end,
		phys_to_virt(mem_info.kernel_end),
		mem_info.linear.start,
		mem_info.linear.end
	);

	unsafe { ZONE_INFO.write(init_linear_map(&mem_info)) };

	unsafe {
		let zone_info = ZONE_INFO.assume_init_ref();
		pr_info!("zone_info: NORMAL: {:?}", zone_info.normal);
		pr_info!("zone_info: HIGH: {:?}", zone_info.high);
		pr_info!("zone_info: SIZE: {}", zone_info.size);
	}

	unsafe { PAGE_ALLOC.write(PageAllocator::new(ZONE_INFO.assume_init_mut())) };

	match cfg!(ktest) {
		true => run_test(),
		false => run_io(),
	};
}

// #[cfg(ktest)]
mod mmtest {
	use crate::mm::{constant::PAGE_SHIFT, Page};

	use super::*;
	use kfs_macro::ktest;

	static mut PAGE_STATE: [bool; usize::MAX >> PAGE_SHIFT] = [false; usize::MAX >> PAGE_SHIFT];
	static mut LCG_SEED: u32 = 42;

	fn lcg_rand() -> u32 {
		unsafe { LCG_SEED = LCG_SEED.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff };

		unsafe { LCG_SEED }
	}

	fn reset_page_state() {
		for x in unsafe { PAGE_STATE.iter_mut() } {
			*x = false;
		}
	}

	fn mark_alloced(addr: usize, rank: usize) {
		let pfn = addr >> PAGE_SHIFT;

		for i in pfn..(pfn + (1 << rank)) {
			unsafe {
				if PAGE_STATE[i] {
					panic!("allocation overwrapped!");
				}
				PAGE_STATE[i] = true;
			}
		}
	}

	fn mark_freed(addr: usize, rank: usize) {
		let pfn = addr >> PAGE_SHIFT;

		for i in pfn..(pfn + (1 << rank)) {
			unsafe {
				if !PAGE_STATE[i] {
					panic!("double free detected.");
				}
				PAGE_STATE[i] = false;
			}
		}
	}

	fn checked_alloc(rank: usize) -> Result<NonNull<Page>, ()> {
		let mem = unsafe { PAGE_ALLOC.assume_init_mut() }.alloc_page(rank)?;

		mark_alloced(mem.as_ptr() as usize, rank);

		Ok(mem)
	}

	fn checked_free(page: NonNull<Page>, rank: usize) {
		mark_freed(page.as_ptr() as usize, rank);

		unsafe { PAGE_ALLOC.assume_init_mut() }.free_page(page);
	}

	#[ktest]
	pub fn min_rank_alloc_free() {
		reset_page_state();

		// allocate untill OOM
		while let Ok(_) = checked_alloc(0) {}

		// free all
		for (i, is_alloced) in unsafe { PAGE_STATE }.iter().enumerate() {
			if *is_alloced {
				checked_free(NonNull::new((i << PAGE_SHIFT) as *mut Page).unwrap(), 0);
			}
		}
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

#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(ascii_char)]
#![feature(allocator_api)]
#![feature(maybe_uninit_uninit_array)]
#![feature(const_maybe_uninit_uninit_array)]
#![feature(new_uninit)]
#![feature(asm_const)]
#![feature(variant_count)]
#![feature(extract_if)]
#![feature(maybe_uninit_as_bytes)]
#![feature(offset_of)]
#![feature(slice_as_chunks)]
#![feature(unboxed_closures)]
#![feature(iter_intersperse)]
#![feature(get_mut_unchecked)]
#![feature(trait_upcasting)]
#![feature(slice_range)]
#![feature(exact_size_is_empty)]

extern crate alloc;

pub mod acpi;
pub mod boot;
pub mod collection;
pub mod config;
pub mod driver;
pub mod elf;
pub mod fs;
pub mod input;
pub mod interrupt;
pub mod io;
pub mod mm;
pub mod net;
pub mod printk;
pub mod process;
pub mod ptr;
pub mod scheduler;
pub mod smp;
pub mod sync;
pub mod syscall;
pub mod test;
pub mod user_bin;
pub mod util;
pub mod x86;

use core::sync::atomic::{AtomicBool, Ordering};
use core::{arch::asm, panic::PanicInfo};
use interrupt::kthread_init;
use process::task::Task;
use scheduler::context::yield_now;
use scheduler::schedule_last;
use scheduler::work::slow_worker;
use test::{exit_qemu_with, TEST_ARRAY};

use crate::driver::apic::apic_timer::jiffies;
use crate::driver::ide::dma::test::TEST_SECTOR_COUNT;
use crate::driver::ide::get_ide_controller;
use crate::driver::ide::ide_id::IdeId;
use crate::interrupt::irq_disable;
use crate::mm::alloc::page::get_available_pages;
use crate::mm::constant::{MB, PAGE_SIZE, SECTOR_SIZE};

pub static RUN_TIME: AtomicBool = AtomicBool::new(false);

/// very simple panic handler.
/// that just print panic infomation and fall into infinity loop.
///
/// we should make sure no more `panic!()` from here.
#[panic_handler]
fn panic_handler_impl(info: &PanicInfo) -> ! {
	irq_disable();
	printk_panic!("{}\ncall stack (most recent call first)\n", info);

	print_stacktrace!();

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

fn run_process() -> ! {
	// let stat = Task::new_kernel(show_page_stat as usize, 0).expect("OOM");
	// schedule_last(stat);

	// let stat = Task::new_kernel(hello_ko as usize, 0).expect("OOM");
	// schedule_last(stat);

	let init = process::get_init_task();
	schedule_last(init);

	let worker = Task::new_kernel(slow_worker as usize, 0).expect("OOM");
	schedule_last(worker);

	mm::oom::init().expect("OOM");

	idle();
}

extern "C" fn show_jiffies(_: usize) {
	loop {
		printk!("\rjiffies: {}", jiffies());
		yield_now();
	}
}

fn idle() -> ! {
	kthread_init();
	loop {
		yield_now();
	}
}

fn halt() {
	unsafe { asm!("hlt") }
}

unsafe fn kernel_boot_alloc(bi_header: usize, magic: u32) {
	let mut bootalloc = boot::init(bi_header, magic).expect("boot information");
	let meta_page_table = mm::page::alloc_meta_page_table(&mut bootalloc);
	bootalloc.deinit();

	mm::page::init_fixed_map();
	driver::apic::local::init().unwrap();
	mm::page::init_arbitrary_map();
	mm::page::init_kernel_pd();

	mm::page::init_metapage_table(meta_page_table);
}

#[no_mangle]
pub fn kernel_entry(bi_header: usize, magic: u32) -> ! {
	driver::serial::init().expect("serial COM1 that will be used at boot time.");

	// caution: order sensitive.
	unsafe { kernel_boot_alloc(bi_header, magic) };

	interrupt::idt::init();
	mm::alloc::page::init();
	mm::alloc::phys::init();
	mm::alloc::virt::init();

	driver::vga::init();
	driver::terminal::init();
	driver::bus::pci::enumerate();

	acpi::init();

	driver::hpet::init().expect("failed to init HPET");
	driver::apic::local::init_timer();
	driver::apic::io::init().expect("IO APIC init.");
	driver::ide::init().expect("IDE controller init.");

	unsafe { x86::init() };

	fs::init_rootfs().expect("failed to mount /");
	process::init();

	fs::init_devfs();
	fs::init_procfs();
	fs::init_sysfs();

	fs::mount_root();

	scheduler::work::init().expect("worker thread init");

	RUN_TIME.store(true, Ordering::Relaxed);
	driver::ide::enable_interrupt();
	driver::serial::ext_init().expect("serial COM1 that will be used at run time.");

	match cfg!(ktest) {
		true => run_test(),
		false => run_process(),
	};
}

mod test_threads {
	use core::alloc::{Allocator, Layout};

	use alloc::{boxed::Box, vec::Vec};

	use crate::{
		mm::{alloc::phys::Normal, constant::KB, oom::wake_up_oom_handler},
		scheduler::preempt::preempt_disable,
	};

	use super::*;

	pub fn run_dma_test(_: usize) {
		let tries = 5;
		let id = unsafe { IdeId::new_unchecked(1) };

		let mut ide = get_ide_controller(id);
		ide.ata.interrupt_pending();

		for i in 0..tries {
			// driver::ide::dma::test::write_dma_event(id, i * 2);
			driver::ide::dma::test::read_dma_event(id, i * 2);
		}
		drop(ide);

		pr_debug!("REQUEST: {} bytes", TEST_SECTOR_COUNT * SECTOR_SIZE * tries);
	}

	pub extern "C" fn show_page_stat(_: usize) -> ! {
		loop {
			let pages = get_available_pages();
			pr_info!("AVAILABLE PAGES: {} ({} MB)", pages, pages * PAGE_SIZE / MB);
			yield_now();
		}
	}

	pub fn leak_test() {
		#[repr(align(1024))]
		struct T;

		loop {
			let mut v = Vec::new();

			for _ in 0..100 {
				let b = Box::new(T);
				v.push(b);
			}

			yield_now();
		}
	}

	pub fn oom_test(_: usize) {
		let layout = unsafe { Layout::from_size_align_unchecked(2 * KB, KB) };
		let mut basket = Vec::new();
		loop {
			let _ = preempt_disable();
			while let Ok(ptr) = Normal.allocate(layout) {
				basket.push(ptr);
			}

			pr_warn!("allocated count: {}", basket.len());

			basket.iter().enumerate().for_each(|(i, ptr)| unsafe {
				if i % 2048 == 0 {
					pr_debug!("dalloc: {} MB", i * 2048 / MB);
				}
				Normal.deallocate(ptr.cast(), layout)
			});

			basket.clear();

			wake_up_oom_handler();
		}
	}
}

pub fn do_something() {
	pr_warn!("hello, world!!!");
}

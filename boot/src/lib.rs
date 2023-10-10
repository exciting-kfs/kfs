#![no_std]
#![allow(dead_code)]

pub use kernel::*;

use core::arch::asm;
use core::sync::atomic::Ordering;
use driver::apic::apic_timer::jiffies;
use driver::ide::dma::test::TEST_SECTOR_COUNT;
use driver::ide::get_ide_controller;
use driver::ide::ide_id::IdeId;
use interrupt::kthread_init;
use mm::alloc::page::get_available_pages;
use mm::constant::{MB, PAGE_SIZE, SECTOR_SIZE};
use process::task::Task;
use scheduler::context::yield_now;
use scheduler::schedule_last;
use scheduler::work::slow_worker;
use test::{exit_qemu_with, TEST_ARRAY};

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
	driver::vga::text_vga::init();
	driver::serial::init().expect("serial COM1 that will be used at boot time.");

	// caution: order sensitive.
	unsafe { kernel_boot_alloc(bi_header, magic) };

	interrupt::idt::init();
	mm::alloc::page::init();
	mm::alloc::phys::init();
	mm::alloc::virt::init();

	driver::terminal::init();
	driver::bus::pci::enumerate();

	acpi::init();

	driver::hpet::init().expect("failed to init HPET");
	driver::apic::local::init_timer();
	driver::apic::io::init().expect("IO APIC init.");
	driver::ps2::init().expect("PS/2 controller init.");
	driver::ide::init().expect("IDE controller init.");

	unsafe { x86::init() };

	fs::init().expect("failed to mount /");
	process::init();

	fs::init_devfs().expect("failed to mount /dev");
	fs::init_procfs().expect("failed to mount /proc");
	fs::ext2::init().expect("failed to mount /ext2");

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
}

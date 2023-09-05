#![no_std]
#![no_main]
#![allow(dead_code)]
#![feature(allocator_api)]
#![feature(maybe_uninit_uninit_array)]
#![feature(const_maybe_uninit_uninit_array)]
#![feature(new_uninit)]
#![feature(asm_const)]
#![feature(variant_count)]
#![feature(extract_if)]
#![feature(maybe_uninit_as_bytes)]
#![feature(slice_as_chunks)]
#![feature(unboxed_closures)]

extern crate alloc;

mod acpi;
mod boot;
mod collection;
mod config;
mod driver;
mod fs;
mod input;
mod interrupt;
mod io;
mod mm;
mod printk;
mod process;
mod ptr;
mod scheduler;
mod smp;
mod sync;
mod syscall;
mod test;
mod user_bin;
mod util;
mod x86;

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};
use core::{arch::asm, panic::PanicInfo};
use driver::tty;
use fs::vfs::{AccessFlag, IOFlag, VfsFileHandle, VfsHandle};
use interrupt::kthread_init;
use process::task::Task;
use scheduler::context::yield_now;
use scheduler::schedule_last;
use scheduler::work::slow_worker;
use test::{exit_qemu_with, TEST_ARRAY};

use crate::driver::ide::dev_num::DevNum;
use crate::driver::ide::dma::test::TEST_SECTOR_COUNT;
use crate::driver::ide::get_ide_controller;
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

fn open_default_fd(task: &mut Arc<Task>) {
	let tty = tty::open(0).unwrap();
	let ext = task.get_user_ext().expect("user task");
	let sess = &ext.lock_relation().get_session();

	tty.lock_tty().connect(Arc::downgrade(sess));
	sess.lock().set_ctty(tty.clone());

	let mut fd_table = ext.lock_fd_table();
	let file = VfsHandle::File(Arc::new(VfsFileHandle::new(
		None,
		Box::new(tty.clone()),
		IOFlag::empty(),
		AccessFlag::O_RDWR,
	)));

	fd_table.alloc_fd(file.clone());
	fd_table.alloc_fd(file.clone());
	fd_table.alloc_fd(file.clone());
}

fn run_process() -> ! {
	// let stat = Task::new_kernel(show_page_stat as usize, 0).expect("OOM");
	// TASK_QUEUE.lock().push_back(stat);
	// let dma_test = Task::new_kernel(test_threads::run_dma_test as usize, 0).expect("OOM");
	// schedule_last(dma_test);

	let worker = Task::new_kernel(slow_worker as usize, 0).expect("OOM");
	let mut init = process::get_init_task();
	open_default_fd(&mut init);

	schedule_last(init);
	schedule_last(worker);

	idle();
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

	driver::console::console_manager::init();
	driver::bus::pci::enumerate();

	acpi::init();

	driver::apic::io::init().expect("IO APIC init.");
	driver::ps2::init().expect("PS/2 controller init.");
	driver::ide::init().expect("IDE controller init.");

	unsafe { x86::init() };
	fs::init().expect("failed to mount /");
	process::init();

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
		let dev_num = unsafe { DevNum::new_unchecked(1) };

		let mut ide = get_ide_controller(dev_num);
		ide.ata.interrupt_pending();

		for i in 0..tries {
			// driver::ide::dma::test::write_dma_event(dev_num, i * 2);
			driver::ide::dma::test::read_dma_event(dev_num, i * 2);
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

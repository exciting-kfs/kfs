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
#![feature(dropck_eyepatch)]
#![feature(maybe_uninit_as_bytes)]
#![feature(slice_as_chunks)]

extern crate alloc;

mod acpi;
mod boot;
mod collection;
mod config;
mod driver;
mod file;
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

use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};
use core::{arch::asm, panic::PanicInfo};
use driver::tty;
use file::{File, OpenFlag};
use process::kthread::kthread_init;
use process::task::Task;
use scheduler::context::yield_now;
use scheduler::work::slow_worker;
use scheduler::TASK_QUEUE;
use test::{exit_qemu_with, TEST_ARRAY};

use crate::interrupt::irq_disable;
use crate::mm::alloc::page::get_available_pages;
use crate::mm::constant::{MB, PAGE_SIZE};

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

fn idle() -> ! {
	kthread_init();
	loop {
		yield_now();
	}
}

fn halt() {
	unsafe { asm!("hlt") }
}

fn open_default_fd(task: &mut Arc<Task>) {
	let tty = tty::open(0).unwrap();
	let ext = task.get_user_ext().expect("user task");
	let sess = &ext.lock_relation().get_session();

	tty.lock().connect(Arc::downgrade(sess));
	sess.lock().set_ctty(tty.clone());

	let mut fd_table = ext.lock_fd_table();
	let file = Arc::new(File {
		ops: tty.clone(),
		open_flag: OpenFlag::O_RDWR,
	});

	fd_table.alloc_fd(file.clone());
	fd_table.alloc_fd(file.clone());
	fd_table.alloc_fd(file.clone());
}

fn run_process() -> ! {
	// let stat = Task::new_kernel(show_page_stat as usize, 0).expect("OOM");
	// TASK_QUEUE.lock().push_back(stat);

	let worker = Task::new_kernel(slow_worker as usize, 0).expect("OOM");
	let mut init = process::get_init_task();
	open_default_fd(&mut init);

	TASK_QUEUE.lock().push_back(init);
	TASK_QUEUE.lock().push_back(worker);

	idle();
}

extern "C" fn repeat_x(x: usize) -> ! {
	loop {
		pr_info!("FROM X={}", x);
		unsafe { asm!("hlt") }
	}
}

extern "C" fn show_page_stat(_: usize) -> ! {
	loop {
		let pages = get_available_pages();
		pr_info!("AVAILABLE PAGES: {} ({} MB)", pages, pages * PAGE_SIZE / MB);
		yield_now();
	}
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

	driver::apic::io::init().expect("io apic init");
	driver::ps2::init().expect("failed to init PS/2");
	driver::ide::init().expect("IDE controller initialization.");

	unsafe { x86::init() };
	process::init();
	scheduler::work::init().expect("worker thread init");

	RUN_TIME.store(true, Ordering::Relaxed);
	driver::serial::ext_init().expect("serial COM1 that will be used at run time.");

	match cfg!(ktest) {
		true => run_test(),
		false => run_process(),
	};
}

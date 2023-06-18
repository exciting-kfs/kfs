use crate::{
	acpi::PROCESSOR_INFO,
	interrupt::{
		apic::local::ipi::{self, Mode, Target, Timeout},
		idt::load_global_idt,
	},
	mm::{
		page::{remap_page_4m, restore_page_4m, PageFlag},
		util::phys_to_virt,
	},
	pr_info,
	util::pit::PIT,
};

extern "C" {
	fn __ap_start();
	fn __ap_end();
	fn AP_START();
	pub static AP_COUNT_VIRT: u8;
}

pub fn init() -> Result<(), Timeout> {
	relocate_ap_start();

	let vaddr = 0x0;
	let flag = PageFlag::Global | PageFlag::Write | PageFlag::Present;
	let backup = remap_page_4m(vaddr, 0, flag);

	wakeup_aps()?;

	restore_page_4m(vaddr, backup);
	Ok(())
}

fn wakeup_aps() -> Result<(), Timeout> {
	ipi::send_then_wait(Target::ExcludeSelf, Mode::INIT, 0)?;

	let count = PROCESSOR_INFO.application_processors.iter().count();
	pr_info!("The number of APs: {}", count);

	for id in 1..(count + 1) {
		let target = ipi::Target::Other(id);
		let mode = ipi::Mode::StartUp;
		let vec_num = (AP_START as usize >> 12) as u8;

		for _ in 0..2 {
			ipi::send_then_wait(target, mode, vec_num)?;
		}
		pr_info!("AP[{}]: init done.", id);
		PIT::wait_ms(10); // to prevent data race.
	}

	while unsafe { AP_COUNT_VIRT } != count as u8 {}
	Ok(())
}

/// relocate `ap.S: __ap_start` to `kernel.ld: AP_START`(physical address)
fn relocate_ap_start() {
	let len = __ap_end as usize - __ap_start as usize;
	let dst = phys_to_virt(AP_START as usize) as *mut u8;
	let src = __ap_start as usize as *const u8;
	unsafe { dst.copy_from_nonoverlapping(src, len) };
}

#[no_mangle]
fn ap_entry(id: usize) {
	load_global_idt();

	pr_info!("AP[{}] is in ap_entry now.", id); // FIXME printk race condition

	// TODO MTRR etc...

	loop {
		unsafe {
			// core::arch::asm!("sti");
			core::arch::asm!("hlt");
		}
	}
}

use core::alloc::Layout;

use crate::{
	acpi::PROCESSOR_INFO,
	interrupt::apic::local::ipi,
	mm::{
		alloc::{phys::allocate, GFP},
		util::{phys_to_virt, size_of_rank},
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

pub fn init() -> Result<(), &'static str> {
	relocate_ap_start();

	let target = ipi::Target::ExcludeSelf;
	let mode = ipi::Mode::INIT;
	ipi::send_then_wait(target, mode, 0).map_err(|_| "timeout ipi INIT")?;

	let count = PROCESSOR_INFO.application_processors.iter().count();
	pr_info!("The number of APs: {}", count);
	for id in 1..(count + 1) {
		let target = ipi::Target::Other(id);
		let mode = ipi::Mode::StartUp;
		let vec_num = (AP_START as usize >> 12) as u8;

		for _ in 0..2 {
			ipi::send_then_wait(target, mode, vec_num).map_err(|_| "timeout ipi Startup")?;
		}
		pr_info!("AP[{}]: init done.", id);
		PIT::wait_ms(10); // to prevent data race.
	}

	while unsafe { AP_COUNT_VIRT } != count as u8 {
		// PIT::wait_ms(35);
		// pr_info!("{}", unsafe { AP_COUNT_VIRT });
	}
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
pub extern "C" fn __ap_stack_alloc() -> *mut u8 {
	let size = size_of_rank(1);
	let layout = unsafe { Layout::from_size_align_unchecked(size, size) };
	let ptr = allocate(layout, GFP::Atomic).expect("ap stack").as_ptr();
	pr_info!("__ap_stack_allocate: {:?}", ptr);
	ptr.cast()
}

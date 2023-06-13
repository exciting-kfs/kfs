use crate::{
	interrupt::{apic::local::ipi, Delay},
	io::pmio::Port,
	mm::util::phys_to_virt,
	pr_info,
};

extern "C" {
	fn AP_START();
	pub static AP_FLAG_VIRT: u8;
}

fn for_break() {}

pub fn init() {
	set_warm_reset_vec(); // ?
	shutdown_cmos(); // ?

	for_break();

	let target = ipi::Target::ExcludeSelf;
	let mode = ipi::Mode::INIT;

	ipi::send_then_wait(target, mode, 0);
	Delay::wait_ms(10);

	let target = ipi::Target::Other(1);
	let mode = ipi::Mode::StartUp;
	let vec_num = (AP_START as usize >> 12) as u8;

	for _ in 0..2 {
		ipi::send_then_wait(target, mode, vec_num);
		Delay::wait_us(200);
	}

	pr_info!("AP_FLAG: {}", unsafe { AP_FLAG_VIRT });
	while unsafe { AP_FLAG_VIRT } == 0 {}
	pr_info!("mp init done.");
}

/// https://wiki.osdev.org/CMOS#Accessing_CMOS_Registers
/// http://www.bioscentral.com/misc/cmosmap.htm
fn shutdown_cmos() {
	let sel = Port::new(0x70);
	let data = Port::new(0x71);

	sel.write_byte(0x80 | 0x0f); // nmi disable | select shutdown status
	data.write_byte(0x0a); // jump double word pointer without EOI / hmm...
}

fn set_warm_reset_vec() {
	let wrv = phys_to_virt(0x40 << 4 | 0x67) as *mut u16;
	let addr = AP_START as usize;
	unsafe {
		wrv.write_unaligned(0);
		wrv.offset(1).write_unaligned((addr >> 4) as u16);
	}
}

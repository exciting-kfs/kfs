pub mod io;
pub mod local;

use crate::io::pmio::Port;
use crate::util::arch::{cpuid::CPUID, msr::Msr};

pub static MSR_APIC_BASE: Msr = Msr::new(0x1b);

pub fn init() {
	if CPUID::run(1, 0).edx & 0x100 != 0x100 {
		panic!("apic unsupported.");
	}

	remap_irq();
	disable_8259_pic();

	local::init().unwrap();
	io::init().unwrap();
}

// TODO wait? study... keyboard interrupt handling
fn disable_8259_pic() {
	let pic_master_data: Port = Port::new(0x21);
	let pic_slave_data: Port = Port::new(0xa1);

	pic_master_data.write_byte(0xff);
	pic_slave_data.write_byte(0xff);
}

// TODO wait? study... keyboard interrupt handling
fn remap_irq() {
	let pic_master_command: Port = Port::new(0x20);
	let pic_slave_command: Port = Port::new(0xa0);
	let pic_master_data: Port = Port::new(0x21);
	let pic_slave_data: Port = Port::new(0xa1);

	pic_master_command.write_byte(0x11);
	pic_slave_command.write_byte(0x11);

	pic_master_data.write_byte(0x20);
	pic_slave_data.write_byte(0x28);

	pic_master_data.write_byte(0x4);
	pic_slave_data.write_byte(0x2);

	pic_master_data.write_byte(0x1);
	pic_slave_data.write_byte(0x1);
}

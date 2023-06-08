mod io;
mod local;

use crate::interrupt::apic::io::REDIR_TABLE_COUNT;
use crate::io::pmio::Port;
use crate::util::arch::cpuid::CPUID;
use crate::{acpi::IOAPIC_INFO, pr_info, sync::singleton::Singleton, util::arch::msr::Msr};

pub use io::pbase as io_pbase;
pub use io::vbase as io_vbase;
pub use local::pbase as local_pbase;
pub use local::vbase as local_vbase;

static MSR_APIC_BASE: Singleton<Msr> = Singleton::new(Msr::new(0x1b));

pub fn init() {
	if CPUID::run(1, 0).edx & 0x100 != 0x100 {
		panic!("apic unsupported.");
	}

	if MSR_APIC_BASE.lock().read().low & 0x800 != 0x800 {
		panic!("apic disabled.");
	}

	if local::Register::SpuriousInterruptVector.read()[0] & 0x100 != 0x100 {
		panic!("apic disabled.");
	}

	remap_irq();
	disable_8259_pic();

	io::init();
}

pub fn print_local() {
	pr_info!("\n**** apic local register ****");
	pr_info!("vbase: {:x}", local::vbase());

	for r in local::Register::iter() {
		pr_info!("{}: {:x?}", r, r.read())
	}
}

pub fn print_io() {
	pr_info!("\n**** apic io register ****");

	for id in 0..IOAPIC_INFO.lock().io_apics.iter().count() {
		pr_info!("vbase: {:x}", io::vbase(id));
		pr_info!("ID: {:x?}", io::read(id, io::RegKind::ID));
		pr_info!("Version: {:x?}", io::read(id, io::RegKind::VERSION));
		pr_info!("ArbID: {:x?}", io::read(id, io::RegKind::ArbitrationID));
		pr_info!("Redirection Table:");

		for i in 0..REDIR_TABLE_COUNT {
			pr_info!(
				"\t {:02}: {:x?}",
				i,
				io::read(id, io::RegKind::RedirectionTable(i as u8))
			);
		}
	}
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

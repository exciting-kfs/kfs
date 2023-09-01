//! PCI IDE Controller Specification.
//! Bus Master IDE Controller.
//! ATA/ATAPI spcification.
//! PIIX specification.
//! OsDev [PCI, PCI IDE Controller, ATA/ATAPI using DMA]
mod bmide;
mod prd;

pub mod block;
pub mod dev_num;
pub mod lba;
pub mod partition;

use core::{array, mem::MaybeUninit, ptr::NonNull};

use kfs_macro::interrupt_handler;

use crate::{
	driver::{apic::local::LOCAL_APIC, bus::pci::header::HeaderType0, ide::bmide::BMIDE},
	interrupt::InterruptFrame,
	mm::{
		alloc::{page::alloc_pages, virt::io_allocate, Zone},
		constant::PAGE_SIZE,
		util::virt_to_phys,
	},
	pr_debug, pr_warn,
	sync::locked::{Locked, LockedGuard},
};

use self::{dev_num::DevNum, prd::PRD};

use super::bus::{
	ata::AtaController,
	pci::{self, find_device, ClassCode},
};

const IDE_CLASS_CODE: ClassCode = ClassCode {
	class: 0x01,
	sub_class: 0x01,
};

static ATA_IDE: [Locked<AtaController>; 2] = [
	Locked::new(AtaController::new(0x1f0, 0x3f6)), // CH: P, DEV: P
	Locked::new(AtaController::new(0x170, 0x376)), // CH: S, DEV: P
];

pub fn init() -> Result<(), pci::Error> {
	// PCI CONFIGURATION SPACE
	let bdf = find_device(IDE_CLASS_CODE)?;
	let h0 = HeaderType0::get(&bdf)?;
	bdf.set_busmaster(true);

	// BUS MASTER IDE
	let bmide_port = match h0.bar4 & 0x1 == 0x1 {
		true => h0.bar4 & 0xffff_fffc,
		false => h0.bar4 & 0xffff_fff0,
	};
	BMIDE::init(bmide_port as u16);

	// PARTITION TABLE
	let existed = array::from_fn(|i| {
		let dev = DevNum::new(i);
		let ide = get_ide_controller(dev);
		let output = ide.self_diagnosis();

		ide.set_interrupt(false);
		(output.error == 0x01).then_some(dev)
	});
	partition::init(existed);

	// test::test_read_dma();
	// test::test_write_dma();

	Ok(())
}

pub fn enable_interrupt() {
	for i in 0..4 {
		let ide = get_ide_controller(DevNum::new(i));
		ide.set_interrupt(true);
	}
}

pub fn get_ide_controller(dev_num: DevNum) -> LockedGuard<'static, AtaController> {
	debug_assert!(dev_num.index() < 4, "invalid ide controller");
	let channel = dev_num.channel();

	let mut ide = ATA_IDE[channel].lock();

	while ide.interrupt_pending() {
		drop(ide);
		ide = ATA_IDE[channel].lock();
	}
	ide.set_device(dev_num);

	ide
}

pub fn get_busmaster_ide(dev_num: DevNum) -> LockedGuard<'static, MaybeUninit<BMIDE>> {
	debug_assert!(dev_num.index() < 4, "invalid ide controller");

	BMIDE[dev_num.channel()].lock()
}

#[interrupt_handler]
pub extern "C" fn handle_ide_impl(_frame: InterruptFrame) {
	const CHANNEL: usize = 0;

	pr_warn!("ide");
	let lock = BMIDE[CHANNEL].lock();
	let bmi = unsafe { lock.assume_init_ref() };

	let mut ide = ATA_IDE[CHANNEL].lock();
	let output = ide.output();

	if bmi.is_error() || output.is_error() {
		let bdf = find_device(IDE_CLASS_CODE).unwrap();
		let h0 = HeaderType0::get(&bdf).unwrap();

		pr_debug!("{}", ide.output());
		pr_debug!("{}", bmi);

		pr_debug!("pci: conf: status: {:x?}", h0.common.status);
		pr_debug!("pci: conf: command: {:x?}", h0.common.command);

		panic!("IDE ERROR"); // TODO is it fine?
	}

	bmi.sync_data();
	bmi.clear();
	bmi.stop();

	ide.interrupt_resolve();

	LOCAL_APIC.end_of_interrupt();
}

pub mod test {

	use crate::driver::ide::lba::LBA28;

	const DEV_NUM: usize = 1;

	use super::*;

	// TODO DELETE
	pub static mut DMA_CHECK: MaybeUninit<NonNull<[u8]>> = MaybeUninit::uninit();
	static mut PRDT: MaybeUninit<PRD> = MaybeUninit::uninit();

	fn clear_paper(page: NonNull<[u8]>) -> &'static mut [u8] {
		let paddr = virt_to_phys(page.as_ptr() as *const usize as usize);
		let mut ptr = io_allocate(paddr, 1).expect("OOM");
		unsafe { DMA_CHECK.write(ptr) };

		let buf = unsafe { ptr.as_mut() };
		for i in 0..PAGE_SIZE {
			buf[i] = 0;
		}
		buf
	}

	fn set_prd_table(bmi: &mut BMIDE, page: NonNull<[u8]>) {
		let paddr = virt_to_phys(page.as_ptr() as *const usize as usize);

		let table = bmi.prd_table();
		table[0] = PRD::new(paddr, 512 as u16);
		table[0].set_eot(true);
	}

	fn test_identify_device(ata_ide: &AtaController) {
		let ata0_id = ata_ide.identify_device();
		pr_debug!("SECTORS: {}", ata0_id.sector_count());
		pr_debug!("MODEL: [{}]", ata0_id.model());
	}

	pub fn test_write_dma() {
		let page = alloc_pages(0, Zone::High).expect("OOM");
		let mut lock = BMIDE[0].lock();
		let bmi = unsafe { lock.assume_init_mut() };
		set_prd_table(bmi, page);
		bmi.set_dma_write();

		pr_debug!("{}", bmi);

		let buf = clear_paper(page);

		"world hello?"
			.as_bytes()
			.iter()
			.enumerate()
			.for_each(|(i, c)| buf[i] = *c);

		// ATA - DO DMA: WRITE DMA
		let mut ide = get_ide_controller(DevNum::new(1));
		ide.write_dma(LBA28::new(0), 1);

		pr_debug!("{}", ide.output());

		bmi.start();

		pr_debug!("{}", bmi);
	}

	pub fn test_read_dma() {
		let page = alloc_pages(0, Zone::High).expect("OOM");
		let mut lock = BMIDE[0].lock();
		let bmi = unsafe { lock.assume_init_mut() };

		set_prd_table(bmi, page);
		bmi.set_dma_read();

		pr_debug!("{}", bmi);

		clear_paper(page);

		// ATA - DO DMA: READ DMA
		let mut ide = get_ide_controller(DevNum::new(1));
		pr_debug!("test_read_dma: {}", ide.output());

		ide.read_dma(LBA28::new(0), 1);
		pr_debug!("{}", ide.output());

		bmi.start();

		pr_debug!("{}", bmi);
	}
}

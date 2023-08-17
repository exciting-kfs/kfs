/// PCI IDE Controller Specification.
/// Bus Master IDE Controller.
/// ATA/ATAPI spcification.
/// PIIX specification.
/// OsDev [PCI, PCI IDE Controller, ATA/ATAPI using DMA]
mod bmide;
mod prd;

use core::{mem::MaybeUninit, ptr::NonNull};

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
	sync::locked::Locked,
};

use self::prd::PRD;

use super::bus::{
	ata::AtaController,
	pci::{self, find_device, ClassCode},
};

const IDE_CLASS_CODE: ClassCode = ClassCode {
	class: 0x01,
	sub_class: 0x01,
};

static ATA_IDE: Locked<[[AtaController; 2]; 2]> = Locked::new([
	// TODO split lock?
	[
		AtaController::new(0x1f0, 0x3f6, false), // CH: P, DEV: P
		AtaController::new(0x1f0, 0x3f6, true),  // CH: P, DEV: S
	],
	[
		AtaController::new(0x170, 0x376, false), // CH: S, DEV: P
		AtaController::new(0x170, 0x376, true),  // CH: S, DEV: S
	],
]);

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

	// test::test_read_dma();
	test::test_write_dma();

	Ok(())
}

pub mod test {
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

	fn set_bmi(bmi: &BMIDE, page: NonNull<[u8]>) {
		let paddr = virt_to_phys(page.as_ptr() as *const usize as usize);

		let prdt = unsafe { PRDT.write(PRD::new(paddr, 512, true)) };
		let paddr = virt_to_phys(prdt as *const PRD as usize);

		bmi.register_prdt(paddr as u32);
		bmi.set_dma_read();
	}

	fn test_identify_device(ata_ide: &AtaController) {
		let ata0_id = ata_ide.identify_device();
		pr_debug!("SECTORS: {}", ata0_id.sector_count());
		pr_debug!("MODEL: [{}]", ata0_id.model());
	}

	pub fn test_write_dma() {
		let page = alloc_pages(0, Zone::High).expect("OOM");

		let bmide = BMIDE.lock();
		let bmi = unsafe { bmide[0].assume_init_ref() };
		set_bmi(bmi, page);

		pr_debug!("{}", bmi);

		let buf = clear_paper(page);

		"world hello?"
			.as_bytes()
			.iter()
			.enumerate()
			.for_each(|(i, c)| buf[i] = *c);

		// ATA - DO DMA: WRITE DMA
		let ata_ide = &ATA_IDE.lock()[0][0];

		ata_ide.write_lba28(0);
		ata_ide.write_sector_count(1);
		ata_ide.write_command(0xca);

		pr_debug!("{}", ata_ide.output());

		test_identify_device(ata_ide);

		bmi.start();

		pr_debug!("{}", bmi);
	}

	pub fn test_read_dma() {
		let page = alloc_pages(0, Zone::High).expect("OOM");
		let bmide = BMIDE.lock();
		let bmi = unsafe { bmide[0].assume_init_ref() };

		set_bmi(bmi, page);

		pr_debug!("{}", bmi);

		clear_paper(page);

		// ATA - DO DMA: READ DMA
		let ata_ide = &ATA_IDE.lock()[0][0];

		ata_ide.write_lba28(0);
		ata_ide.write_sector_count(1);
		ata_ide.write_command(0xc8);

		pr_debug!("{}", ata_ide.output());

		test_identify_device(ata_ide);

		bmi.start();

		pr_debug!("{}", bmi);
	}
}

#[interrupt_handler]
pub extern "C" fn handle_ide_impl(_frame: InterruptFrame) {
	pr_warn!("ide");
	let bmide = BMIDE.lock();
	let bmi = unsafe { bmide[0].assume_init_ref() };

	let ata_ide = &ATA_IDE.lock()[0][0];
	let ata_out = ata_ide.output();

	if bmi.is_error() || ata_out.is_error() {
		pr_debug!("{}", ata_ide.output());
		pr_debug!("{}", bmi);

		let bdf = find_device(IDE_CLASS_CODE).unwrap();
		let h0 = HeaderType0::get(&bdf).unwrap();

		pr_debug!("pci: conf: status: {:x?}", h0.common.status);
		pr_debug!("pci: conf: command: {:x?}", h0.common.command);

		panic!("IDE ERROR"); // TODO is it fine?
	}

	bmi.sync_data();
	bmi.clear();
	bmi.stop();

	LOCAL_APIC.end_of_interrupt();
}

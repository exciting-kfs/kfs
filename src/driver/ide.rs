//! PCI IDE Controller Specification.
//! Bus Master IDE Controller.
//! ATA/ATAPI spcification.
//! PIIX specification.
//! OsDev [PCI, PCI IDE Controller, ATA/ATAPI using DMA]
mod bmide;
mod prd;

pub mod block;
pub mod dev_num;
pub mod dma;
pub mod handler;
pub mod lba;
pub mod partition;

use core::{array, mem::MaybeUninit, ptr::NonNull};

use crate::{
	driver::ide::bmide::BMIDE,
	mm::{
		alloc::{page::alloc_pages, virt::io_allocate, Zone},
		constant::PAGE_SIZE,
		util::virt_to_phys,
	},
	pr_debug,
	scheduler::context::yield_now,
	sync::{
		locked::{Locked, LockedGuard},
		TryLockFail,
	},
};

use self::{dev_num::DevNum, prd::PRD};

use super::{
	apic::io::{set_irq_mask, IDE_PRIMARY_IRQ, IDE_SECONDARY_IRQ},
	bus::{
		ata::AtaController,
		pci::{self, ClassCode},
	},
};

const IDE_CLASS_CODE: ClassCode = ClassCode {
	class: 0x01,
	sub_class: 0x01,
};

pub /* TODO for test*/ static IDE: [Locked<IdeController>; 2] = [
	Locked::new(IdeController::new(AtaController::new(0x1f0, 0x3f6))), // CH: P
	Locked::new(IdeController::new(AtaController::new(0x170, 0x376))), // CH: S
];

pub struct IdeController {
	pub ata: AtaController,
	pub bmi: MaybeUninit<BMIDE>,
}

impl IdeController {
	const fn new(ata: AtaController) -> Self {
		Self {
			ata,
			bmi: MaybeUninit::uninit(),
		}
	}
}

pub fn init() -> Result<(), pci::Error> {
	let bmide = BMIDE::for_each_channel()?;

	IDE.iter().zip(bmide).for_each(|(ide, bmi)| unsafe {
		let in_ide = &mut ide.lock().bmi;
		in_ide.write(bmi);
		in_ide.assume_init_mut().load_prd_table();
	});

	// PART TABLE
	let existed = array::from_fn(|i| {
		let dev = DevNum::new(i);
		let ide = get_ide_controller(dev);
		let output = ide.ata.self_diagnosis();

		ide.ata.set_interrupt(false);
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
		ide.ata.set_interrupt(true);
	}

	set_irq_mask(IDE_PRIMARY_IRQ, false).expect("ide irq");
	set_irq_mask(IDE_SECONDARY_IRQ, false).expect("ide irq");
}

pub fn get_ide_controller(dev_num: DevNum) -> LockedGuard<'static, IdeController> {
	debug_assert!(dev_num.index() < 4, "invalid ide controller");
	let channel = dev_num.channel();

	let mut ide = IDE[channel].lock();

	while !ide.ata.is_idle() {
		drop(ide);
		yield_now();
		ide = IDE[channel].lock();
	}

	ide.ata.set_device(dev_num);

	ide
}

pub fn try_get_ide_controller(
	dev_num: DevNum,
	try_count: usize,
) -> Result<LockedGuard<'static, IdeController>, TryLockFail> {
	debug_assert!(dev_num.index() < 4, "invalid ide controller");
	let channel = dev_num.channel();

	let mut ide = IDE[channel].lock();
	let mut count = 0;

	while count < try_count {
		if ide.ata.is_idle() {
			ide.ata.set_device(dev_num);
			return Ok(ide);
		}
		drop(ide);
		yield_now();
		ide = IDE[channel].lock();
		count += 1;
	}

	Err(TryLockFail)
}

pub mod test {

	use crate::driver::{
		bus::ata::Command,
		ide::{dma::DmaOps, lba::LBA28},
	};

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
		let mut ide = get_ide_controller(DevNum::new(1));
		let bmi = unsafe { ide.bmi.assume_init_mut() };
		set_prd_table(bmi, page);
		bmi.set_dma(DmaOps::Write);

		pr_debug!("{}", bmi);

		let buf = clear_paper(page);

		"world hello?"
			.as_bytes()
			.iter()
			.enumerate()
			.for_each(|(i, c)| buf[i] = *c);

		// ATA - DO DMA: WRITE DMA
		ide.ata.do_dma(Command::WriteDma, LBA28::new(0), 1);

		pr_debug!("{}", ide.ata.output());

		let bmi = unsafe { ide.bmi.assume_init_mut() };
		bmi.start();

		pr_debug!("{}", bmi);
	}

	pub fn test_read_dma() {
		let page = alloc_pages(0, Zone::High).expect("OOM");
		let mut ide = get_ide_controller(DevNum::new(1));
		let bmi = unsafe { ide.bmi.assume_init_mut() };

		set_prd_table(bmi, page);
		bmi.set_dma(DmaOps::Read);

		pr_debug!("{}", bmi);

		clear_paper(page);

		// ATA - DO DMA: READ DMA
		pr_debug!("test_read_dma: {}", ide.ata.output());

		ide.ata.do_dma(Command::ReadDma, LBA28::new(0), 1);
		pr_debug!("{}", ide.ata.output());

		let bmi = unsafe { ide.bmi.assume_init_mut() };
		bmi.start();

		pr_debug!("{}", bmi);
	}
}

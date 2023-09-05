use kfs_macro::interrupt_handler;

use crate::{
	driver::{
		apic::local::LOCAL_APIC,
		bus::pci::{find_device, header::HeaderType0},
		ide::IDE_CLASS_CODE,
	},
	interrupt::InterruptFrame,
	pr_debug, pr_warn,
	scheduler::work::schedule_slow_work,
};

use super::{dma::dma_q::work, ide_id::IdeId, IDE};

#[interrupt_handler]
pub extern "C" fn handle_ide_ch0_impl(_frame: InterruptFrame) {
	pr_warn!("ide ch 0");
	handle_ide_impl(0);
}

#[interrupt_handler]
pub extern "C" fn handle_ide_ch1_impl(_frame: InterruptFrame) {
	pr_warn!("ide ch 1");
	handle_ide_impl(1);
}

pub fn handle_ide_impl(channel: usize) {
	let mut ide = IDE[channel].lock();

	let output = ide.ata.output();
	let is_secondary = output.is_secondary();
	let bmi = unsafe { ide.bmi.assume_init_mut() };

	if bmi.is_error() || output.is_error() {
		// TODO retry..?
		let bdf = find_device(IDE_CLASS_CODE).unwrap();
		let h0 = HeaderType0::get(&bdf).unwrap();

		pr_debug!("{}", bmi);
		pr_debug!("{}", ide.ata.output());

		pr_debug!("pci: conf: status: {:x?}", h0.common.status);
		pr_debug!("pci: conf: command: {:x?}", h0.common.command);

		panic!("IDE ERROR");
	} else {
		// schedule work.
		let num = channel * 2 + (is_secondary as usize);
		let id = unsafe { IdeId::new_unchecked(num) };
		schedule_slow_work(work::do_next_dma, id);
		pr_debug!("ide handler: do_next_dma scheduled: {:?}", id);
	}

	bmi.sync_data();
	bmi.clear();
	bmi.stop();

	ide.ata.interrupt_resolve();

	LOCAL_APIC.end_of_interrupt();
}

pub mod dma_q;
pub mod event;

use core::alloc::AllocError;

use crate::{driver::ide::dma::dma_q::work, pr_debug, scheduler::work::schedule_slow_work};

use self::{dma_q::get_dma_q, event::Event};

use super::dev_num::DevNum;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DmaOps {
	Read,
	Write,
}

pub fn dma_schedule(dev_num: DevNum, event: Event) -> Result<(), AllocError> {
	let mut dma_q = get_dma_q(dev_num);

	if dma_q.is_idle() {
		dma_q.push_front(dev_num, event);
		dma_q.schedule_next();
		schedule_slow_work(work::do_dma, dev_num);
		pr_debug!("dma_schedule: do_dma scheduled");
	} else {
		dma_q.merge_insert(dev_num, event);
		pr_debug!("DMA_Q[{}].len: {:?}", dev_num.channel(), dma_q.len());
	}
	Ok(())
}

pub mod test {
	use alloc::boxed::Box;

	use crate::{
		driver::ide::{
			block::{self, Block},
			dev_num::DevNum,
			dma::{dma_schedule, Event},
			lba::LBA28,
		},
		pr_debug, printk,
	};

	use super::{event::CallBack, *};

	pub const TEST_SECTOR_COUNT: usize = 128;

	pub fn write_dma_event(dev_num: DevNum, i: usize) {
		let begin = LBA28::new(i * TEST_SECTOR_COUNT);
		let end = LBA28::new((i + 1) * TEST_SECTOR_COUNT);

		let prepare = move || {
			pr_debug!("+++++ prepare called +++++");
			Block::new(block::Size::Sector(TEST_SECTOR_COUNT)).map(|block| unsafe {
				let mut block: Block<[u8]> = block.into();
				let arr = block.as_slice(block.size()).as_mut();
				arr.iter_mut().for_each(|e| *e = b'b' + i as u8);
				block.into()
			})
		};

		let mut cb = CallBack::new();
		cb.prologue.insert(begin, Box::new(prepare));
		cb.epilogue.insert(
			begin,
			Box::new(move |_| {
				pr_debug!("+++++ cleanup called +++++");
			}),
		);

		let dma = Event::new(DmaOps::Write, begin, end, cb);
		dma_schedule(dev_num, dma).expect("OOM");
	}

	pub fn read_dma_event(dev_num: DevNum, i: usize) {
		let begin = LBA28::new(i * TEST_SECTOR_COUNT);
		let end = LBA28::new((i + 1) * TEST_SECTOR_COUNT);

		let prepare = move || {
			pr_debug!("+++++ prepare called +++++");
			Block::new(block::Size::Sector(TEST_SECTOR_COUNT)).map(|block| block.into())
		};

		let mut cb = CallBack::new();

		cb.prologue.insert(begin, Box::new(prepare));
		cb.epilogue.insert(
			begin,
			Box::new(move |block| {
				pr_debug!("+++++ cleanup called +++++");

				let mut block = block.into::<[u8]>();
				let slice = unsafe { block.as_slice(block.size()) };

				for i in 0..10 {
					printk!("{:x}", slice[i]);
				}
				pr_debug!("");
			}),
		);

		let dma = Event::new(DmaOps::Read, begin, end, cb);
		dma_schedule(dev_num, dma).expect("OOM");
	}
}

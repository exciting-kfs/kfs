pub mod dma_q;
pub mod event;
pub mod read;
pub mod write;

use core::alloc::AllocError;

use crate::pr_debug;

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
		dma_q.start_with(dev_num, event);
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

	use super::{event::CallBack, read::ReadDma, write::WriteDma};

	pub const TEST_SECTOR_COUNT: usize = 128;

	pub fn write_dma_event(dev_num: DevNum, i: usize) {
		let begin = unsafe { LBA28::new_unchecked(i * TEST_SECTOR_COUNT) };
		let end = unsafe { LBA28::new_unchecked((i + 1) * TEST_SECTOR_COUNT) };

		let prepare = move || {
			pr_debug!("+++++ prepare called +++++");
			let size = block::BlockSize::from_sector_count(TEST_SECTOR_COUNT).unwrap();
			Block::new(size).map(|block| unsafe {
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

		let dma = WriteDma::new(begin, end, cb);
		dma_schedule(dev_num, Event::Write(dma)).expect("OOM");
	}

	pub fn read_dma_event(dev_num: DevNum, i: usize) {
		let begin = unsafe { LBA28::new_unchecked(i * TEST_SECTOR_COUNT) };
		let end = unsafe { LBA28::new_unchecked((i + 1) * TEST_SECTOR_COUNT) };

		let prepare = move || {
			pr_debug!("+++++ prepare called +++++");
			let size = block::BlockSize::from_sector_count(TEST_SECTOR_COUNT).unwrap();
			Block::new(size).map(|block| block.into())
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

		let dma = ReadDma::new(begin, end, cb);
		dma_schedule(dev_num, Event::Read(dma)).expect("OOM");
	}
}

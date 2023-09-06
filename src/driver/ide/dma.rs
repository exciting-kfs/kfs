mod call_back;
mod dma_req;

pub mod dma_q;
pub mod event;

use core::alloc::AllocError;

use crate::pr_debug;

use self::{dma_q::get_dma_q, event::DmaInit};

use super::dev_num::DevNum;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DmaOps {
	Read,
	Write,
}

pub fn dma_schedule(dev_num: DevNum, event: DmaInit) -> Result<(), AllocError> {
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
	use core::alloc::AllocError;

	use alloc::boxed::Box;

	use crate::{
		driver::ide::{
			block::{self, Block},
			dev_num::DevNum,
			dma::{dma_schedule, DmaInit},
			lba::LBA28,
		},
		pr_debug, printk,
	};

	use super::{call_back::CallBack, dma_req::ReqInit};

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

		let cb = CallBack::new(
			begin,
			Box::new(prepare),
			Box::new(move |_| {
				pr_debug!("+++++ cleanup called +++++");
			}),
		);

		let dma = ReqInit::new(begin..end, cb);
		dma_schedule(dev_num, DmaInit::Write(dma)).expect("OOM");
	}

	pub fn read_dma_event(dev_num: DevNum, i: usize) {
		let begin = unsafe { LBA28::new_unchecked(i * TEST_SECTOR_COUNT) };
		let end = unsafe { LBA28::new_unchecked((i + 1) * TEST_SECTOR_COUNT) };

		let prepare = move || {
			pr_debug!("+++++ prepare called +++++");
			let size = block::BlockSize::from_sector_count(TEST_SECTOR_COUNT).unwrap();
			Block::new(size).map(|block| block.into())
		};

		let cleanup = move |block: Result<Block, AllocError>| {
			pr_debug!("+++++ cleanup called +++++");

			let mut block = block.expect("OOM").into::<[u8]>();
			let slice = unsafe { block.as_slice(block.size()) };

			for i in 0..10 {
				printk!("{:x}", slice[i]);
			}
			pr_debug!("");
		};

		let cb = CallBack::new(begin, Box::new(prepare), Box::new(cleanup));
		let dma = ReqInit::new(begin..end, cb);
		dma_schedule(dev_num, DmaInit::Read(dma)).expect("OOM");
	}
}

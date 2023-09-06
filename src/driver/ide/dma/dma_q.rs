use core::{
	alloc::AllocError,
	mem::{replace, take},
};

use alloc::collections::LinkedList;

use crate::{
	driver::ide::{dev_num::DevNum, IdeController},
	pr_debug,
	scheduler::work::schedule_slow_work,
	sync::locked::{Locked, LockedGuard},
};

use super::{dma_req::ReqInit, event::DmaRun, DmaInit};

static DMA_Q: [Locked<DmaQ>; 2] = [Locked::new(DmaQ::new(0)), Locked::new(DmaQ::new(1))];

pub struct DmaQ {
	prev: DevNum,
	scheduled: Option<DmaRun>,
	queue: [LinkedList<DmaInit>; 2],
}

impl DmaQ {
	pub const fn new(channel: usize) -> Self {
		Self {
			prev: unsafe { DevNum::new_unchecked(channel * 2) },
			scheduled: None,
			queue: [LinkedList::new(), LinkedList::new()],
		}
	}

	pub fn is_idle(&self) -> bool {
		!(self.scheduled.is_some() || self.queue[0].len() > 0 || self.queue[1].len() > 0)
	}

	pub fn do_next(&mut self, ide: LockedGuard<'_, IdeController>) -> Result<(), AllocError> {
		let running = match self.pop_front() {
			Some(next) => {
				let ready = next.prepare()?;
				let running = ready.perform(ide);
				Some(running)
			}
			None => None,
		};

		let prev = replace(&mut self.scheduled, running);

		if let Some(ev) = prev {
			ev.cleanup();
		}

		Ok(())
	}

	pub fn retry(&mut self, ide: LockedGuard<'_, IdeController>) {
		let ev = take(&mut self.scheduled);

		if let Some(running) = ev {
			let ready = running.ready();
			let running = ready.perform(ide);
			self.scheduled = Some(running)
		}
	}

	// for debug
	pub fn len(&self) -> [usize; 2] {
		let mut ret = [0, 0];
		self.queue
			.iter()
			.enumerate()
			.for_each(|(i, q)| ret[i] = q.iter().count());
		ret
	}

	pub fn start_with(&mut self, dev: DevNum, event: DmaInit) {
		self.prev = dev.pair();
		self.queue[dev.index_in_channel()].push_front(event);
		schedule_slow_work(work::do_next_dma, dev);
		pr_debug!("dma_schedule: start_dma scheduled");
	}

	pub fn merge_insert(&mut self, dev: DevNum, event: DmaInit) {
		let index = dev.index_in_channel();
		let queue = &mut self.queue[index];

		let merge_condition = |in_q: &&mut DmaInit| match (in_q, &event) {
			(DmaInit::Write(in_q), DmaInit::Write(req)) => ReqInit::can_merge(in_q, &req),
			(DmaInit::Read(in_q), DmaInit::Read(req)) => ReqInit::can_merge(in_q, &req),
			_ => false,
		};

		if let Some(e) = queue.iter_mut().find(|e| merge_condition(e)) {
			let (_, req) = event.divide();
			e.inner().merge(req)
		} else {
			queue.push_back(event);
		}
	}

	fn pop_front(&mut self) -> Option<DmaInit> {
		let pair = self.prev.pair();
		let pair_i = pair.index_in_channel();
		let prev_i = self.prev.index_in_channel();

		if let Some(ev) = self.queue[pair_i].pop_front() {
			self.prev = pair;
			Some(ev)
		} else if let Some(ev) = self.queue[prev_i].pop_front() {
			Some(ev)
		} else {
			None
		}
	}
}

/// # Caution
///
/// - lock order: ide - dma_q
pub fn get_dma_q<'a>(dev_num: DevNum) -> LockedGuard<'a, DmaQ> {
	DMA_Q[dev_num.channel()].lock()
}

pub mod work {
	use crate::{
		driver::ide::{
			dev_num::DevNum, dma::dma_q::get_dma_q, get_ide_controller, try_get_ide_controller,
		},
		printk,
		scheduler::work::Error,
	};

	const LOCK_TRY: usize = 3;

	pub fn retry_dma(dev_num: &mut DevNum) -> Result<(), Error> {
		let ide = get_ide_controller(*dev_num);
		let mut dma_q = get_dma_q(*dev_num);

		dma_q.retry(ide);
		Ok(())
	}

	pub fn do_next_dma(dev_num: &mut DevNum) -> Result<(), Error> {
		printk!(".");
		let ide = try_get_ide_controller(*dev_num, LOCK_TRY).map_err(|_| Error::Yield)?;
		let mut dma_q = get_dma_q(*dev_num);

		dma_q.do_next(ide).map_err(|_| Error::AllocError)
	}
}

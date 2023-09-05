use core::alloc::AllocError;

use alloc::collections::LinkedList;

use crate::{
	driver::ide::{dev_num::DevNum, IdeController},
	pr_debug,
	sync::locked::{Locked, LockedGuard},
};

use super::{DmaOps, Event};

static DMA_Q: [Locked<DmaQ>; 2] = [Locked::new(DmaQ::new()), Locked::new(DmaQ::new())];

pub struct DmaQ {
	prev: DevNum,
	scheduled: Option<Event>,
	queue: [LinkedList<Event>; 2],
}

impl DmaQ {
	pub const fn new() -> Self {
		Self {
			prev: unsafe { DevNum::new_unchecked(0) },
			scheduled: None,
			queue: [LinkedList::new(), LinkedList::new()],
		}
	}

	pub fn is_idle(&self) -> bool {
		self.scheduled.is_none()
	}

	pub fn schedule_next(&mut self) {
		pr_debug!("schedule next: len {:?}", self.len());
		let next = self.pop_front();
		let prev = core::mem::replace(&mut self.scheduled, next);

		if let Some(ev) = prev {
			ev.cleanup();
		}
	}

	pub fn retry_scheduled(&mut self, ide: LockedGuard<'_, IdeController>) {
		let ev = self
			.scheduled
			.as_mut()
			.expect("already scheduled dma event");
		ev.retry(ide);
	}

	pub fn do_scheduled(&mut self, ide: LockedGuard<'_, IdeController>) -> Result<(), AllocError> {
		if let Some(ev) = self.scheduled.as_mut() {
			let blocks = ev.prepare()?;
			ev.perform(ide, blocks);
		}
		Ok(())
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

	pub fn push_front(&mut self, dev: DevNum, event: Event) {
		self.prev = dev;
		self.queue[dev.index_in_channel()].push_front(event);
	}

	pub fn merge_insert(&mut self, dev: DevNum, event: Event) {
		let index = dev.index_in_channel();
		let queue = &mut self.queue[index];

		let merge_condition = |in_q: &&mut Event| match (in_q.kind, event.kind) {
			(DmaOps::Write, DmaOps::Write) => {
				(in_q.kilo_bytes() + event.kilo_bytes() <= Event::MAX_KB)
					&& (in_q.begin == event.end || in_q.end == event.begin)
			}
			(DmaOps::Read, DmaOps::Read) => {
				(in_q.kilo_bytes() + event.kilo_bytes() <= Event::MAX_KB)
					&& (in_q.begin == event.end || in_q.end == event.begin)
			}
			_ => false,
		};

		if let Some(e) = queue.iter_mut().find(|e| merge_condition(e)) {
			e.merge(event);
		} else {
			queue.push_back(event);
		}
	}

	fn pop_front(&mut self) -> Option<Event> {
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
		pr_warn, printk,
		scheduler::work::Error,
	};

	const LOCK_TRY: usize = 3;

	pub fn do_dma(dev_num: &mut DevNum) -> Result<(), Error> {
		printk!(".");
		let ide = try_get_ide_controller(*dev_num, LOCK_TRY).map_err(|_| Error::Yield)?;
		let mut dma_q = get_dma_q(*dev_num);

		dma_q.do_scheduled(ide).map_err(|_| Error::AllocError)
	}

	pub fn retry_dma(dev_num: &mut DevNum) -> Result<(), Error> {
		let ide = get_ide_controller(*dev_num);
		let mut dma_q = get_dma_q(*dev_num);

		dma_q.retry_scheduled(ide);
		Ok(())
	}

	pub fn do_next_dma(dev_num: &mut DevNum) -> Result<(), Error> {
		pr_warn!("do next dma work: {:?}", dev_num);
		let ide = try_get_ide_controller(*dev_num, LOCK_TRY).map_err(|_| Error::Yield)?;
		let mut dma_q = get_dma_q(*dev_num);

		dma_q.schedule_next();
		dma_q.do_scheduled(ide).map_err(|_| Error::AllocError)
	}
}

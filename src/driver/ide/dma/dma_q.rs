use core::mem::{replace, take};

use alloc::collections::LinkedList;

use crate::{
	driver::ide::{ide_id::IdeId, IdeController},
	scheduler::{context::yield_now, work::schedule_slow_work},
	sync::{Locked, LockedGuard},
	trace_feature,
};

use super::{event::DmaRun, DmaInit};

static DMA_Q: [Locked<DmaQ>; 2] = [Locked::new(DmaQ::new(0)), Locked::new(DmaQ::new(1))];

pub struct DmaQ {
	prev: IdeId,
	scheduled: Option<DmaRun>,
	queue: [LinkedList<DmaInit>; 2],
}

impl DmaQ {
	pub const fn new(channel: usize) -> Self {
		Self {
			prev: unsafe { IdeId::new_unchecked(channel * 2) },
			scheduled: None,
			queue: [LinkedList::new(), LinkedList::new()],
		}
	}

	pub fn is_idle(&self) -> bool {
		!(self.scheduled.is_some() || self.queue[0].len() > 0 || self.queue[1].len() > 0)
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

	pub fn start_with(&mut self, dev: IdeId, event: DmaInit) {
		self.prev = dev.pair();
		self.queue[dev.index_in_channel()].push_front(event);
		schedule_slow_work(work::do_next_dma, dev);
	}

	pub fn merge_insert(&mut self, dev: IdeId, mut event: DmaInit) {
		let index = dev.index_in_channel();
		let queue = &mut self.queue[index];

		if let Some(_) = queue
			.front_mut()
			.and_then(|inq| inq.try_merge(&mut event).ok())
		{
			return;
		}

		if let Some(_) = queue
			.back_mut()
			.and_then(|inq| inq.try_merge(&mut event).ok())
		{
			return;
		}

		queue.push_back(event);
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

	fn take_scheduled(&mut self) -> Option<DmaRun> {
		take(&mut self.scheduled)
	}

	fn scheduled(&mut self, running: DmaRun) {
		trace_feature!(
			"time-dma-verbose",
			"start: {}",
			crate::driver::ide::dma::dma_q::get_timestamp_micro() % 1_000_000,
		);

		let _ = replace(&mut self.scheduled, Some(running));
	}

	fn retry(&mut self, ide: LockedGuard<'_, IdeController>) {
		let ev = take(&mut self.scheduled);

		if let Some(running) = ev {
			let ready = running.ready();
			let running = ready.perform(ide);
			self.scheduled = Some(running)
		}
	}
}

/// # Caution
///
/// - lock order: ide - dma_q
pub fn get_dma_q<'a>(id: IdeId) -> LockedGuard<'a, DmaQ> {
	DMA_Q[id.channel()].lock()
}

pub fn wait_idle() {
	while !(DMA_Q[0].lock().is_idle() && DMA_Q[1].lock().is_idle()) {
		yield_now();
	}
}

pub mod work {
	use core::mem::take;

	use alloc::boxed::Box;

	use crate::{
		driver::ide::{
			dma::{dma_q::get_dma_q, event::DmaReady},
			get_ide_controller,
			ide_id::IdeId,
			try_get_ide_controller,
		},
		scheduler::work::{default::DefaultWork, Error},
		trace_feature,
	};

	const LOCK_TRY: usize = 3;

	pub fn retry_dma(id: &mut IdeId) -> Result<(), Error> {
		let ide = get_ide_controller(*id);
		let mut dma_q = get_dma_q(*id);

		dma_q.retry(ide);
		Ok(())
	}

	pub fn do_next_dma(id: &mut IdeId) -> Result<(), Error> {
		trace_feature!(
			"time-dma-verbose",
			"do_next_dma: {}",
			crate::driver::ide::dma::dma_q::get_timestamp_micro() % 1_000_000,
		);

		let (scheduled, event) = {
			let mut dma_q = get_dma_q(*id);
			(dma_q.take_scheduled(), dma_q.pop_front())
		};

		if let Some(ev) = scheduled {
			trace_feature!(
				"time-dma-verbose",
				"end: {}",
				crate::driver::ide::dma::dma_q::get_timestamp_micro() % 1_000_000,
			);
			ev.cleanup();
		}

		// first, hold read lock of the `Arc<LockRW<Block>>`.
		let ready = event
			.map(|ev| ev.prepare())
			.transpose()
			.map_err(|_| Error::Alloc)?;

		// second, hold try lock of the `ide`.
		let ide = match try_get_ide_controller(*id, LOCK_TRY) {
			Ok(ide) => ide,
			Err(_) => {
				let arg = Box::new((*id, ready));
				let work = DefaultWork::new(do_next_dma_postponed, arg);
				return Err(Error::Next(Box::new(work)));
			}
		};

		// last, hold lock of the `dma_q`
		if let Some(running) = ready.map(|ready| ready.perform(ide)) {
			let mut dma_q = get_dma_q(*id);
			dma_q.scheduled(running);
		}

		Ok(())
	}

	pub fn do_next_dma_postponed(arg: &mut (IdeId, Option<DmaReady>)) -> Result<(), Error> {
		// crate:: pr_warn!("do next dma postponed");
		let id = &mut arg.0;
		let ide = try_get_ide_controller(*id, LOCK_TRY).map_err(|_| Error::Retry)?;

		let ready = take(&mut arg.1);
		let running = ready.map(|ready| ready.perform(ide));
		let mut dma_q = get_dma_q(*id);

		if let Some(running) = running {
			dma_q.scheduled(running);
		}

		Ok(())
	}
}

use core::alloc::AllocError;

use crate::{driver::ide::IdeController, pr_debug, sync::LockedGuard};

use super::{
	dma_req::{ReqInit, ReqReady},
	DmaOps,
};

pub enum DmaInit {
	Read(ReqInit),
	Write(ReqInit),
}

impl DmaInit {
	pub const MAX_KB: usize = 128;

	pub fn prepare(self) -> Result<DmaReady, AllocError> {
		let (ops, inner) = self.divide();
		let ReqInit { range, cb } = inner;

		match cb.prepare() {
			Ok((blocks, cleanup)) => {
				let req = ReqReady {
					range,
					blocks,
					cleanup,
				};
				Ok(match ops {
					DmaOps::Read => DmaReady::Read(req),
					DmaOps::Write => DmaReady::Write(req),
				})
			}
			Err(mut cleanup) => {
				cleanup.iter_mut().for_each(|cb| cb(Err(AllocError)));
				Err(AllocError)
			}
		}
	}

	pub(super) fn inner(&mut self) -> &mut ReqInit {
		match self {
			DmaInit::Read(req) => req,
			DmaInit::Write(req) => req,
		}
	}

	pub(super) fn divide(self) -> (DmaOps, ReqInit) {
		match self {
			DmaInit::Read(req) => (DmaOps::Read, req),
			DmaInit::Write(req) => (DmaOps::Write, req),
		}
	}
}

pub enum DmaReady {
	Read(ReqReady),
	Write(ReqReady),
}

impl DmaReady {
	pub fn perform(self, mut ide: LockedGuard<'_, IdeController>) -> DmaRun {
		pr_debug!("+++++ perform called +++++");
		let (ops, inner) = self.divide();

		// (write)cache writeback for blocks
		let bmi = unsafe { ide.bmi.assume_init_mut() };
		bmi.set_prd_table(&inner.blocks);
		bmi.set_dma(ops);

		let ata = &mut ide.ata;
		ata.do_dma(ops, inner.range.start, inner.count() as u16);

		unsafe { ide.bmi.assume_init_mut().start() };

		match ops {
			DmaOps::Read => DmaRun::Read(inner),
			DmaOps::Write => DmaRun::Write(inner),
		}
	}

	fn divide(self) -> (DmaOps, ReqReady) {
		match self {
			DmaReady::Read(req) => (DmaOps::Read, req),
			DmaReady::Write(req) => (DmaOps::Write, req),
		}
	}
}

pub enum DmaRun {
	Read(ReqReady),
	Write(ReqReady),
}

impl DmaRun {
	pub fn cleanup(self) {
		let inner = self.inner();

		let ReqReady {
			range: _,
			blocks: own,
			cleanup,
		} = inner;

		own.into_iter()
			.zip(cleanup)
			.for_each(|(block, mut cb)| cb(Ok(block)))
	}

	pub fn ready(self) -> DmaReady {
		match self {
			DmaRun::Read(req) => DmaReady::Read(req),
			DmaRun::Write(req) => DmaReady::Write(req),
		}
	}

	fn inner(self) -> ReqReady {
		match self {
			DmaRun::Read(req) => req,
			DmaRun::Write(req) => req,
		}
	}
}

use core::{alloc::AllocError, ops::Range};

use crate::{
	driver::ide::{bmide::BMIDE, lba::LBA28, IdeController},
	sync::LockedGuard,
};

use super::{
	dma_req::{ReqInit, ReqReady, ReqWBInit, ReqWBReady},
	DmaOps,
};

pub enum DmaInit {
	Read(ReqInit),
	Write(ReqInit),
	WriteBack(ReqWBInit),
}

impl DmaInit {
	pub const MAX_KB: usize = 128;

	pub fn prepare(self) -> Result<DmaReady, AllocError> {
		match self {
			Self::Read(req) => Self::prepare_own(req, DmaOps::Read),
			Self::Write(req) => Self::prepare_own(req, DmaOps::Write),
			Self::WriteBack(req) => Ok(Self::prepare_wb(req)),
		}
	}

	pub fn try_merge(&mut self, mut other: &mut Self) -> Result<(), ()> {
		use DmaInit::*;
		match (self, &mut other) {
			(Read(in_q), Read(req)) | (Write(in_q), Write(req)) => {
				ReqInit::can_merge(in_q, req).then(|| in_q.merge(req))
			}
			(WriteBack(in_q), WriteBack(req)) => {
				ReqWBInit::can_merge(in_q, &req).then(|| in_q.merge(req))
			}
			_ => None,
		}
		.ok_or(())
	}

	fn prepare_own(req: ReqInit, ops: DmaOps) -> Result<DmaReady, AllocError> {
		let ReqInit { range, cb } = req;

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

	fn prepare_wb(req: ReqWBInit) -> DmaReady {
		let ReqWBInit { range, cb } = req;

		let (blocks, cleanup) = cb.prepare();
		let req = ReqWBReady {
			range,
			blocks,
			cleanup,
		};

		DmaReady::WriteBack(req)
	}
}

pub enum DmaReady {
	Read(ReqReady),
	Write(ReqReady),
	WriteBack(ReqWBReady),
}

impl DmaReady {
	pub fn perform(self, mut ide: LockedGuard<'_, IdeController>) -> DmaRun {
		let ops = self.operation();
		let range = self.range();
		let count = self.count();

		let bmi = unsafe { ide.bmi.assume_init_mut() };
		self.set_prd_table(bmi);
		bmi.set_dma(ops);

		let ata = &mut ide.ata;
		ata.do_dma(ops, range.start, count as u16);
		unsafe { ide.bmi.assume_init_mut().start() };

		self.into()
	}

	fn set_prd_table(&self, bmi: &mut BMIDE) {
		match self {
			Self::Read(req) => bmi.set_prd_table(&req.blocks),
			Self::Write(req) => {
				// (write)cache writeback for blocks
				bmi.set_prd_table(&req.blocks);
			}
			Self::WriteBack(req) => {
				// (write)cache writeback for blocks
				bmi.set_prd_table_wb(&req.blocks)
			}
		}
	}

	fn range(&self) -> Range<LBA28> {
		match self {
			Self::Read(req) | Self::Write(req) => req.range.clone(),
			Self::WriteBack(req) => req.range.clone(),
		}
	}

	fn count(&self) -> usize {
		let range = self.range();
		range.end - range.start
	}

	fn operation(&self) -> DmaOps {
		match self {
			Self::Read(_) => DmaOps::Read,
			Self::WriteBack(_) | Self::Write(_) => DmaOps::Write,
		}
	}
}

impl From<DmaRun> for DmaReady {
	fn from(value: DmaRun) -> Self {
		match value {
			DmaRun::Read(req) => DmaReady::Read(req),
			DmaRun::Write(req) => DmaReady::Write(req),
			DmaRun::WriteBack(req) => DmaReady::WriteBack(req),
		}
	}
}

pub enum DmaRun {
	Read(ReqReady),
	Write(ReqReady),
	WriteBack(ReqWBReady),
}

impl DmaRun {
	pub fn ready(self) -> DmaReady {
		self.into()
	}

	pub fn cleanup(self) {
		match self {
			Self::Read(req) | Self::Write(req) => Self::cleanup_own(req),
			Self::WriteBack(req) => Self::cleanup_ref(req),
		}
	}

	fn cleanup_own(req: ReqReady) {
		let ReqReady {
			range: _,
			blocks,
			cleanup,
		} = req;

		blocks
			.into_iter()
			.zip(cleanup)
			.for_each(|(block, mut cb)| cb(Ok(block)))
	}

	fn cleanup_ref(req: ReqWBReady) {
		let ReqWBReady {
			range: _,
			blocks,
			cleanup,
		} = req;

		blocks
			.into_iter()
			.zip(cleanup)
			.for_each(|(block, mut cb)| cb(block))
	}
}

impl From<DmaReady> for DmaRun {
	fn from(value: DmaReady) -> Self {
		match value {
			DmaReady::Read(req) => DmaRun::Read(req),
			DmaReady::Write(req) => DmaRun::Write(req),
			DmaReady::WriteBack(req) => DmaRun::WriteBack(req),
		}
	}
}

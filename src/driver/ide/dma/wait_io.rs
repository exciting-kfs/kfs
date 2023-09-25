use core::alloc::AllocError;

use alloc::{collections::BTreeMap, sync::Arc};

use crate::{
	driver::ide::block::Block,
	process::{
		relation::Pid,
		task::{Task, CURRENT},
	},
	scheduler::sleep::{sleep_and_yield, wake_up, Sleep},
	sync::Locked,
};

pub struct WaitIO {
	io_result: Locked<BTreeMap<Pid, Result<Block, AllocError>>>,
}

impl WaitIO {
	pub fn new() -> Self {
		Self {
			io_result: Locked::new(BTreeMap::new()),
		}
	}

	pub fn submit(&self, target: &Arc<Task>, io_result: Result<Block, AllocError>) {
		self.io_result.lock().insert(target.get_pid(), io_result);
		wake_up(target, Sleep::Deep);
	}

	pub fn wait(&self) -> Result<Block, AllocError> {
		let current = unsafe { CURRENT.get_mut() }.clone();

		sleep_and_yield(Sleep::Deep);

		let pid = current.get_pid();
		self.io_result
			.lock()
			.remove(&pid)
			.expect("lost dma io request")
	}
}

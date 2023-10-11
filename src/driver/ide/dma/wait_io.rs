use core::alloc::AllocError;

use alloc::{collections::BTreeMap, sync::Arc};

use crate::{
	driver::ide::block::Block,
	process::{
		relation::Pid,
		signal::poll_signal_queue,
		task::{Task, CURRENT},
	},
	scheduler::sleep::{sleep_and_yield, wake_up},
	scheduler::{
		preempt::AtomicOps,
		sleep::{sleep_and_yield_atomic, Sleep},
	},
	sync::Locked,
	syscall::errno::Errno,
};

#[derive(Debug)]
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

	pub fn wait(&self, atomic: AtomicOps) -> Result<Block, Errno> {
		let current = unsafe { CURRENT.get_mut() }.clone();
		let pid = current.get_pid();

		sleep_and_yield_atomic(Sleep::Light, atomic);

		loop {
			if let Some(result) = self.io_result.lock().remove(&pid) {
				return result.map_err(|_| Errno::ENOMEM);
			}

			sleep_and_yield(Sleep::Light);
			unsafe { poll_signal_queue() }?;
		}
	}
}

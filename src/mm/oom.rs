use core::{alloc::AllocError, mem::MaybeUninit};

use alloc::sync::Arc;

use crate::{
	fs::ext2,
	process::task::Task,
	scheduler::{
		schedule_last,
		sleep::{sleep_and_yield, wake_up_deep_sleep, Sleep},
	},
};

use super::{alloc::page::get_available_pages, constant::OOM_WATER_MARK};

static mut OOM_HANDLER: MaybeUninit<Arc<Task>> = MaybeUninit::uninit();

pub fn init() -> Result<(), AllocError> {
	let task = Task::new_kernel(oom_handler as usize, 0)?;

	unsafe { OOM_HANDLER.write(task.clone()) };
	schedule_last(task);

	Ok(())
}

pub fn wake_up_oom_handler() {
	wake_up_deep_sleep(unsafe { OOM_HANDLER.assume_init_ref() });
}

pub fn oom_handler(_: usize) {
	loop {
		sleep_and_yield(Sleep::Deep);

		while get_available_pages() < OOM_WATER_MARK {
			ext2::oom_handler();
		}
	}
}

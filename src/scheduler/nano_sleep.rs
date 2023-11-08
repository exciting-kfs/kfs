use core::mem::replace;

use alloc::{
	collections::BTreeMap,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{
	driver::hpet::get_timestamp_nano,
	fs::vfs::TimeSpec,
	mm::user::verify::{verify_ptr, verify_ptr_mut},
	process::{
		signal::poll_signal_queue,
		task::{Task, CURRENT},
	},
	scheduler::sleep::Sleep,
	sync::Locked,
	syscall::errno::Errno,
};

use super::sleep::{sleep_and_yield_lock, wake_up_weak};

pub static ALARM: Locked<Alarm> = Locked::new(Alarm::new());

pub struct Alarm {
	by_time: BTreeMap<u64, Vec<Weak<Task>>>,
}

impl Alarm {
	const fn new() -> Self {
		Self {
			by_time: BTreeMap::new(),
		}
	}

	fn register(&mut self, target_time: u64) {
		let current = unsafe { CURRENT.get_ref() };

		self.by_time
			.entry(target_time)
			.and_modify(|v| v.push(Arc::downgrade(current)))
			.or_insert({
				let mut v = Vec::new();

				v.push(Arc::downgrade(current));
				v
			});
	}

	pub fn wake_up(&mut self) {
		let current_time = get_timestamp_nano();

		let not = self.by_time.split_off(&current_time);
		let elapsed = replace(&mut self.by_time, not);

		elapsed
			.into_iter()
			.flat_map(|(_, v)| v.into_iter())
			.for_each(|w| wake_up_weak(w, Sleep::Light));
	}
}

pub fn sys_nanosleep(req: usize, rem: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let req = verify_ptr::<TimeSpec>(req, current)?;

	let mut alarm = ALARM.lock();

	alarm.register(req.nano());

	sleep_and_yield_lock(Sleep::Light, alarm);

	let time = req
		.nano()
		.checked_sub(get_timestamp_nano())
		.unwrap_or_default();

	if rem != 0 {
		let remain = verify_ptr_mut::<TimeSpec>(rem, current)?;
		*remain = TimeSpec::from(time);
	}

	unsafe { poll_signal_queue() }.map(|_| 0)
}

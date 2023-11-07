use core::mem::transmute;

use crate::{
	driver::hpet::{get_time_elapsed, get_timestamp_nano},
	fs::vfs::TimeSpec,
	mm::user::verify::verify_ptr_mut,
	process::task::CURRENT,
};

use super::errno::Errno;

#[repr(u8)]
enum ClockId {
	Realtime = 0,
	Monotonic = 1,
}

impl ClockId {
	fn from_usize(value: usize) -> Option<Self> {
		match value {
			x @ 0..=1 => Some(unsafe { transmute(x as u8) }),
			_ => None,
		}
	}
}

pub fn sys_clock_gettime(clk_id: usize, tp: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };
	let clk_id = ClockId::from_usize(clk_id).ok_or(Errno::EINVAL)?;
	let tp = verify_ptr_mut::<TimeSpec>(tp, current)?;

	*tp = match clk_id {
		ClockId::Realtime => TimeSpec::from(get_timestamp_nano()),
		ClockId::Monotonic => TimeSpec::from(get_time_elapsed()),
	};

	Ok(0)
}

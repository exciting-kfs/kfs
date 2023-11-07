use core::{
	cell::UnsafeCell,
	ptr::{addr_of_mut, NonNull},
	sync::atomic::Ordering,
};

use crate::{
	acpi::HPET_BASE, driver::rtc::get_timestamp_utc, mm::constant::HIGH_IO_OFFSET, pr_info,
	RUN_TIME,
};

#[repr(packed)]
struct HpetRegisters {
	capabilities: u64,
	_rsvd1: u64,
	configuration: u64,
	_rsvd2: u64,
	interrupt_status: u64,
	_rsvd3: [u64; 25],
	counter_l: u32,
	counter_h: u32,
	timers: [HpetTimerRegister; 0],
}

#[repr(packed)]
struct HpetTimerRegister {
	_rsvd: u64,
	configuration: u64,
	comparator: u64,
	interrupt_route: u64,
}

pub static HPET: Hpet = Hpet::uninit();

macro_rules! addr_of_reg {
	($reg:ident) => {
		addr_of_mut!(((*HPET.reg_ptr()).$reg))
	};
}

macro_rules! read_reg {
	($reg:ident) => {
		unsafe { addr_of_reg!($reg).read_volatile() }
	};
}

macro_rules! write_reg {
	($reg:ident, $value:expr) => {
		unsafe { addr_of_reg!($reg).write_volatile($value) }
	};
}

pub struct Hpet {
	base: UnsafeCell<NonNull<HpetRegisters>>,
}

unsafe impl Sync for Hpet {}

impl Hpet {
	const fn uninit() -> Self {
		Self {
			base: UnsafeCell::new(NonNull::dangling()),
		}
	}

	unsafe fn init(&self, ptr: NonNull<HpetRegisters>) {
		self.base.get().write(ptr);
	}

	fn reg_ptr(&self) -> *mut HpetRegisters {
		unsafe { (*self.base.get()).as_ptr() }
	}

	/// get femto-seconds per clock
	pub fn clock_speed(&self) -> u32 {
		(read_reg!(capabilities) >> 32) as u32
	}

	pub fn is_counter_64bit(&self) -> bool {
		((read_reg!(capabilities) >> 13) & 1) == 1
	}

	pub fn nr_timers(&self) -> u32 {
		(((read_reg!(capabilities) >> 8) & 0b11111) + 1) as u32
	}

	pub fn enable_counter(&self) {
		let old = read_reg!(configuration);
		write_reg!(configuration, old | 1);
	}

	pub fn get_counter(&self) -> u64 {
		loop {
			let high1 = read_reg!(counter_h);
			let low = read_reg!(counter_l);
			let high2 = read_reg!(counter_h);
			if high1 == high2 {
				return (high1 as u64) << 32 | low as u64;
			}
		}
	}
}

#[derive(Debug)]
pub enum HpetInitError {
	InvalidCounterSize,
	InvalidBase,
}

static mut BOOT_TIMESTAMP: u64 = 0;

pub fn init() -> Result<(), HpetInitError> {
	let hpet_base = unsafe { HPET_BASE };

	if hpet_base < HIGH_IO_OFFSET {
		return Err(HpetInitError::InvalidBase);
	}

	let ptr = NonNull::new(hpet_base as *mut HpetRegisters).ok_or(HpetInitError::InvalidBase)?;

	unsafe { HPET.init(ptr) };

	let clock_speed = HPET.clock_speed();
	let nr_timers = HPET.nr_timers();
	let is_counter_64bit = HPET.is_counter_64bit();

	pr_info!("HPET: {} femto seconds per clock", clock_speed);
	pr_info!("HPET: {} TIMERS available", nr_timers);
	pr_info!("HPET: 64bit COUNTER: {}", is_counter_64bit);

	if !is_counter_64bit {
		return Err(HpetInitError::InvalidCounterSize);
	}

	unsafe { BOOT_TIMESTAMP = get_timestamp_utc() * 1_000_000_000 };
	HPET.enable_counter();

	Ok(())
}

#[inline]
pub fn get_time_elapsed() -> u64 {
	HPET.get_counter() * (HPET.clock_speed() as u64 / 1_000_000)
}

pub fn get_timestamp_nano() -> u64 {
	unsafe { BOOT_TIMESTAMP + get_time_elapsed() }
}

pub fn get_timestamp_mili() -> u64 {
	get_timestamp_nano() / 1000 / 1000
}

pub fn get_timestamp_micro() -> u64 {
	if RUN_TIME.load(Ordering::Relaxed) {
		get_timestamp_nano() / 1000
	} else {
		0
	}
}

pub fn get_timestamp_second() -> u64 {
	get_timestamp_nano() / 1000 / 1000 / 1000
}

use super::Register;

use crate::{
	mm::constant::MB,
	pr_info,
	util::{arch::cpuid::CPUID, bitrange::BitRange},
};

const MASK: BitRange = BitRange::new(16, 17);
const MODE: BitRange = BitRange::new(17, 19);

pub fn init(freq: usize, mode: Mode, vec_num: u8) {
	set_frequency(freq);

	let mut v = Register::LvtTimer.read();

	v &= !(MASK.mask() as u32); // enable timer.
	v |= MODE.fit(mode as usize) as u32;
	v |= vec_num as u32;
	Register::LvtTimer.write(v);
}

pub fn set_frequency(freq: usize) {
	let cpuid = CPUID::run(0x16, 0);

	let bus_freq = (cpuid.ecx & 0xffff) * MB;
	let count = bus_freq / freq;

	pr_info!("Bus freqeuncy(MHz): {}", cpuid.ecx);
	pr_info!("Timer interrupt freqeuncy(Hz): {}", freq);
	pr_info!("count: {}", count);

	let mut div_conf = Register::DivideConfiguration.read();
	div_conf = div_conf | 0b1011; // divided by 1 (bus_freq / n)
	Register::DivideConfiguration.write(div_conf);
	Register::InitialCount.write(count as u32);
}

pub fn freqeuncy() -> usize {
	let cpuid = CPUID::run(0x16, 0);
	let bus_freq = (cpuid.ecx & 0xffff) * MB;
	let count = Register::InitialCount.read() as usize;

	bus_freq / count
}

#[repr(u8)]
pub enum Mode {
	OneShot = 0b00,
	Periodic = 0b01,
	TSC = 0b10,
}

use crate::util::{bitrange::BitRange, pit::PIT};

use super::{read_interrupt_command, write_interrupt_command};

#[derive(Debug)]
pub struct Timeout;

#[derive(Debug, Clone, Copy)]
pub enum Target {
	ItSelf,
	Other(usize),
	ExcludeSelf,
	All,
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
	Fixed = 0b000,
	LowestPriority = 0b001,
	SMI = 0b010,
	NMI = 0b100,
	INIT = 0b101,
	StartUp = 0b110,
}

const H_DEST_FIELD: BitRange = BitRange::new(24, 32);
const L_DEST_SHORTHAND: BitRange = BitRange::new(18, 20);
const L_TRIGGER_MODE: BitRange = BitRange::new(15, 16);
const L_ASSERT: BitRange = BitRange::new(14, 15);
const L_DELIVERY_STATUS: BitRange = BitRange::new(12, 13);
const L_DELIVERY_MODE: BitRange = BitRange::new(8, 11);

fn fill_common(buf: &mut [u32; 2], target: Target, mode: Mode, vec_num: u8) {
	let shorthand = L_DEST_SHORTHAND.fit(match target {
		Target::Other(_) => 0b00,
		Target::ItSelf => 0b01,
		Target::All => 0b10,
		Target::ExcludeSelf => 0b11,
	}) as u32;

	let dest_field = H_DEST_FIELD.fit(match target {
		Target::Other(n) => n,
		Target::ItSelf => 0,
		_ => 0xff, // p6?
	}) as u32;

	let delivery_mode = L_DELIVERY_MODE.fit(mode as usize) as u32;

	buf[0] = delivery_mode | shorthand | vec_num as u32;
	buf[1] = dest_field;
}

pub fn send(target: Target, mode: Mode, vec_num: u8) {
	let mut buf: [u32; 2] = [0, 0];

	fill_common(&mut buf, target, mode, vec_num);

	buf[0] |= L_ASSERT.mask() as u32;
	write_interrupt_command(&buf);
}

pub fn wait() -> Result<(), Timeout> {
	let mut us: usize = 0;
	let interval = 20;
	let deadline = 200;
	while let Status::Pending = read_status() {
		if us >= deadline {
			return Err(Timeout);
		}
		PIT::wait_us(interval);
		us += interval;
	}
	Ok(())
}

pub fn send_then_wait(target: Target, mode: Mode, vec_num: u8) -> Result<(), Timeout> {
	send(target, mode, vec_num);
	wait()
}

pub fn send_level_deassert(target: Target, mode: Mode, vec_num: u8) {
	let mut buf: [u32; 2] = [0, 0];

	fill_common(&mut buf, target, mode, vec_num);

	buf[0] |= L_TRIGGER_MODE.mask() as u32;

	write_interrupt_command(&buf);
}

pub fn send_level(target: Target, mode: Mode, vec_num: u8) {
	let mut buf: [u32; 2] = [0, 0];

	fill_common(&mut buf, target, mode, vec_num);

	buf[0] |= L_TRIGGER_MODE.mask() as u32;
	buf[0] |= L_ASSERT.mask() as u32;

	write_interrupt_command(&buf);
}

pub enum Status {
	Idle,
	Pending,
}

pub fn read_status() -> Status {
	let mut buf: [u32; 2] = [0, 0];
	let mask = L_DELIVERY_STATUS.mask() as u32;

	read_interrupt_command(&mut buf);

	match buf[0] & mask == mask {
		false => Status::Idle,
		true => Status::Pending,
	}
}

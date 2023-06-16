use crate::io::pmio::Port;

pub struct PIT;

// The easiest method for the timings is to use the PIT's mode 0.
// Write 0x30 to IO port 0x43 (select mode 0 for counter 0),
// then write your count value to 0x40,
// LSB first (e.g. write 0xA9 then 0x4 for a millisecond).

// To check if counter has finished,
// write 0xE2 to IO port 0x43,
// then read a status byte from port 0x40.
// If the 7th bit is set, then it has finished.

impl PIT {
	fn wait(mut count: usize) {
		let pit_sel: Port = Port::new(0x30);
		let pit_data: Port = Port::new(0x40);
		let check_start: Port = Port::new(0x43);

		pit_sel.write_byte(0x43);
		for _ in 0..2 {
			pit_data.write_byte(count as u8);
			count = count >> 8;
		}

		check_start.write_byte(0xe2);
		while pit_data.read_byte() & 0x10 == 0 {}
	}

	pub fn wait_ms(ms: usize) {
		let one_ms = 0x4a9;
		if ms > 0xffff / one_ms {
			panic!("too large");
		}

		Self::wait(ms * one_ms);
	}

	pub fn wait_us(us: usize) {
		if us > 0xffff {
			panic!("too large");
		}

		Self::wait(us);
	}
}

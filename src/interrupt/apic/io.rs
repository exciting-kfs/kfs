use alloc::vec::Vec;

use crate::{acpi::IOAPIC_INFO, mm::util::phys_to_virt, sync::locked::Locked};

pub const REDIR_TABLE_COUNT: usize = 24;

static mut IOAPIC_ACCESS_REGISTERS: Vec<Locked<AccessRegister>> = Vec::new();

struct AccessRegister {
	base_addr: usize,
}

impl AccessRegister {
	const SELECT: usize = 0x00;
	const WINDOW: usize = 0x10;

	const fn new(base_addr: usize) -> Self {
		Self { base_addr }
	}

	fn read(&self, kind: RegKind) -> Vec<usize> {
		match kind {
			x @ (RegKind::ID | RegKind::VERSION | RegKind::ArbitrationID) => {
				self.read_register(x.identify(), 1)
			}
			x => self.read_register(x.identify(), 2),
		}
	}

	fn write(&self, kind: RegKind, value: Vec<usize>) {
		match kind {
			x @ (RegKind::ID | RegKind::RedirectionTable(_)) => {
				self.write_register(x.identify(), value)
			}
			_ => panic!("write operation unavailable."),
		}
	}

	fn read_register(&self, reg_id: usize, count: usize) -> Vec<usize> {
		(0..count)
			.map(|i| {
				self.select_register(reg_id + i);
				self.read_window()
			})
			.collect()
	}

	fn write_register(&self, reg_id: usize, value: Vec<usize>) {
		value.iter().enumerate().for_each(|(i, v)| {
			self.select_register(reg_id + i);
			self.write_window(*v);
		})
	}

	fn select_register(&self, reg_id: usize) {
		let addr = self.base_addr + Self::SELECT;
		let ptr = addr as *mut u8;
		unsafe { ptr.write(reg_id as u8) };
	}

	fn read_window(&self) -> usize {
		let addr = self.base_addr + Self::WINDOW;
		let ptr = addr as *mut usize;
		unsafe { ptr.read() }
	}

	fn write_window(&self, value: usize) {
		let addr = self.base_addr + Self::WINDOW;
		let ptr = addr as *mut usize;
		unsafe { ptr.write(value) };
	}
}

#[derive(Clone)]
pub enum RegKind {
	ID,
	VERSION,
	ArbitrationID,
	RedirectionTable(u8),
}

impl RegKind {
	fn identify(&self) -> usize {
		match self.clone() {
			Self::ID => 0x00,
			Self::VERSION => 0x01,
			Self::ArbitrationID => 0x02,
			Self::RedirectionTable(x) => {
				if x >= REDIR_TABLE_COUNT as u8 {
					panic!("invalid RedirectionTable index");
				}
				(x as usize) * 2 + 0x10
			}
		}
	}
}

pub fn init() {
	for io_apic in IOAPIC_INFO.io_apics.iter() {
		unsafe {
			let base_addr = phys_to_virt(io_apic.address as usize);
			IOAPIC_ACCESS_REGISTERS.push(Locked::new(AccessRegister::new(base_addr)));
		}
	}

	// setting keyboard interrupt vector.

	// FIXME: uncomment here
	let mut v = read(0, RegKind::RedirectionTable(1));
	v[0] = 0x21;
	write(0, RegKind::RedirectionTable(1), v)
}

pub fn read(ioapic_id: usize, kind: RegKind) -> Vec<usize> {
	unsafe { IOAPIC_ACCESS_REGISTERS[ioapic_id].lock().read(kind) }
}

pub fn write(ioapic_id: usize, kind: RegKind, value: Vec<usize>) {
	unsafe { IOAPIC_ACCESS_REGISTERS[ioapic_id].lock().write(kind, value) }
}

pub fn pbase(ioapic_id: usize) -> usize {
	IOAPIC_INFO.io_apics[ioapic_id].address as usize
}

pub fn vbase(ioapic_id: usize) -> usize {
	unsafe { IOAPIC_ACCESS_REGISTERS[ioapic_id].lock().base_addr }
}

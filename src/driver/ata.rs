// = 0xf10

use alloc::{string::String, vec::Vec};

use crate::{
	io::pmio::Port,
	pr_info,
	util::bitrange::{BitData, BitRange},
};

struct AtaController {
	command: Port,
	control: Port,
	is_secondary: bool,
}

#[repr(transparent)]
struct RawSector([u16; 256]);

impl AtaController {
	const DATA: u16 = 0;
	const ERROR_FEATURES: u16 = 1;
	const SECTOR_COUNT: u16 = 2;
	const LBA_LOW: u16 = 3;
	const LBA_MID: u16 = 4;
	const LBA_HIGH: u16 = 5;
	const DEVICE: u16 = 6;
	const STATUS_COMMAND: u16 = 7;

	const LBA_LOW_RANGE: BitRange = BitRange::new(0, 8);
	const LBA_MID_RANGE: BitRange = BitRange::new(8, 16);
	const LBA_HIGH_RANGE: BitRange = BitRange::new(16, 24);
	const LBA_TOP_RANGE: BitRange = BitRange::new(24, 28);

	pub const fn new(command_base: u16, control_base: u16, is_secondary: bool) -> Self {
		Self {
			command: Port::new(command_base),
			control: Port::new(control_base),
			is_secondary,
		}
	}

	fn write_lba28(&self, lba: usize) {
		let lba = BitData::new(lba);

		self.command
			.add(Self::LBA_LOW)
			.write_byte(lba.shift_get_bits(&Self::LBA_LOW_RANGE) as u8);

		self.command
			.add(Self::LBA_MID)
			.write_byte(lba.shift_get_bits(&Self::LBA_MID_RANGE) as u8);

		self.command
			.add(Self::LBA_HIGH)
			.write_byte(lba.shift_get_bits(&Self::LBA_HIGH_RANGE) as u8);

		self.command.add(Self::DEVICE).write_byte(
			lba.shift_get_bits(&Self::LBA_TOP_RANGE) as u8
				| ((self.is_secondary as u8) << 4)
				| (1 << 6),
		)
	}

	/// Perform READ SECTORS command (PIO)
	pub fn read_sectors(&self, lba: usize, buf: &mut [RawSector]) {
		let sector_count = buf.len() as u8;

		self.command
			.add(Self::SECTOR_COUNT)
			.write_byte(sector_count);

		self.write_lba28(lba);

		self.command.add(Self::STATUS_COMMAND).write_byte(0x20);

		for sector in buf {
			for word in &mut sector.0 {
				*word = self.command.add(Self::DATA).read_u16();
			}
		}
	}

	pub fn identify_device(&self) -> AtaId {
		self.command
			.add(Self::DEVICE)
			.write_byte((self.is_secondary as u8) << 4);

		self.command.add(Self::STATUS_COMMAND).write_byte(0xec);

		let mut data = RawSector([0; 256]);
		for word in &mut data.0 {
			*word = self.command.add(Self::DATA).read_u16();
		}

		AtaId { data }
	}
}

struct AtaId {
	data: RawSector,
}

impl AtaId {
	pub fn sector_count(&self) -> u32 {
		unsafe { *(&self.data.0[60] as *const u16).cast::<u32>() }
	}

	pub fn model(&self) -> String {
		let v: Vec<u8> = self.data.0[27..47]
			.iter()
			.flat_map(|ch| ch.to_be_bytes())
			.collect();

		String::from_utf8(v).unwrap()
	}
}

pub fn test() {
	let ata0 = AtaController::new(0x1f0, 0x3f6, false);
	// let ata1 = AtaController::new(0x1f0, 0x3f6, true);
	// let ata2 = AtaController::new(0x170, 0x376, false);
	// let ata3 = AtaController::new(0x170, 0x376, true);

	let ata0_id = ata0.identify_device();

	pr_info!("SECTORS: {}", ata0_id.sector_count());
	pr_info!("MODEL: [{}]", ata0_id.model());
}

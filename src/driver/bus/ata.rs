use core::{array, fmt::Display, mem::transmute};

use alloc::{string::String, vec::Vec};

use crate::{
	io::pmio::Port,
	util::bitrange::{BitData, BitRange},
};

pub struct AtaController {
	command: Port,
	control: Port,
	is_2nd_dev: bool,
}

#[repr(transparent)]
pub struct RawSector([u16; 256]);

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

	pub const fn new(command_base: u16, control_base: u16, is_2nd_dev: bool) -> Self {
		Self {
			command: Port::new(command_base),
			control: Port::new(control_base),
			is_2nd_dev,
		}
	}

	pub fn write_lba28(&self, lba: usize) {
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
				| ((self.is_2nd_dev as u8) << 4)
				| (1 << 6),
		)
	}

	pub fn write_sector_count(&self, count: u8) {
		self.command.add(Self::SECTOR_COUNT).write_byte(count);
	}

	pub fn write_command(&self, command: Command) {
		self.command
			.add(Self::STATUS_COMMAND)
			.write_byte(command as u8);
	}

	pub fn output(&self) -> AtaOutput {
		let off: [u16; 7] = array::from_fn(|i| (i + 1) as u16);
		let res = off.map(|o| self.command.add(o).read_byte());

		unsafe { transmute(res) }
	}

	/// Perform READ SECTORS command (PIO)
	pub fn read_sectors(&self, lba: usize, buf: &mut [RawSector]) {
		let sector_count = buf.len() as u8;

		self.command
			.add(Self::SECTOR_COUNT)
			.write_byte(sector_count);

		self.write_lba28(lba);

		self.command
			.add(Self::STATUS_COMMAND)
			.write_byte(Command::ReadSectors as u8);

		for sector in buf {
			for word in &mut sector.0 {
				*word = self.command.add(Self::DATA).read_u16();
			}
		}
	}

	pub fn identify_device(&self) -> AtaId {
		self.command
			.add(Self::DEVICE)
			.write_byte((self.is_2nd_dev as u8) << 4);

		self.command
			.add(Self::STATUS_COMMAND)
			.write_byte(Command::IdentifyDevice as u8);

		let mut data = RawSector([0; 256]);
		for word in &mut data.0 {
			*word = self.command.add(Self::DATA).read_u16();
		}

		AtaId { data }
	}
}

pub struct AtaId {
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

#[repr(C)]
#[derive(Debug)]
pub struct AtaOutput {
	pub error: u8,
	pub sector_count: u8,
	pub lba_low: u8,
	pub lba_mid: u8,
	pub lba_high: u8,
	pub device: u8,
	pub status: u8,
}

impl AtaOutput {
	fn lba(&self) -> u32 {
		(AtaController::LBA_TOP_RANGE.fit(self.device as usize)
			+ AtaController::LBA_HIGH_RANGE.fit(self.lba_high as usize)
			+ AtaController::LBA_MID_RANGE.fit(self.lba_mid as usize)
			+ AtaController::LBA_LOW_RANGE.fit(self.lba_low as usize)) as u32
	}

	pub fn is_error(&self) -> bool {
		self.status & 0x1 == 0x1
	}
}

impl Display for AtaOutput {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let dev = match self.device & (1 << 4) == (1 << 4) {
			true => "secondary",
			false => "primary",
		};

		write!(f, "[ATA OUTPUT]\n")?;
		write!(f, "err: 0b{:b}\n", self.error)?;
		write!(f, "dev: {}\n", dev)?;
		write!(f, "lba: 0x{:x}\n", self.lba())?;
		write!(f, "sector count: 0x{:x}\n", self.sector_count)?;

		Ok(())
	}
}

#[repr(u8)]
pub enum Command {
	ReadDMA = 0xc8,
	WriteDMA = 0xca,
	ReadSectors = 0x20,
	IdentifyDevice = 0xec,
}

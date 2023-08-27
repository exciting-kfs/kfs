use core::{array, fmt::Display, mem::transmute, ops::Deref};

use alloc::{string::String, vec::Vec};

use crate::{
	driver::ide::lba::LBA28,
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

impl RawSector {
	pub fn new(buf: [u16; 256]) -> Self {
		Self(buf)
	}
}

impl Deref for RawSector {
	type Target = [u16; 256];
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

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

	const SIG_BUSY: u8 = 0x80;
	const SIG_DRQ: u8 = 0x08;

	pub const fn new(command_base: u16, control_base: u16, is_2nd_dev: bool) -> Self {
		Self {
			command: Port::new(command_base),
			control: Port::new(control_base),
			is_2nd_dev,
		}
	}

	pub fn write_lba28(&self, lba: LBA28) {
		let lba = BitData::new(lba.as_raw());

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

	// TODO Runtime: yield
	pub fn wait_command(&self) {
		let mut status = self.control.read_byte();
		while status & 0x80 > 0 || status & 0x08 > 0 {
			status = self.control.read_byte();
		}
	}

	pub fn write_command(&self, command: Command) {
		self.wait_command();
		self.command
			.add(Self::STATUS_COMMAND)
			.write_byte(command as u8);
	}

	pub fn output(&self) -> AtaOutput {
		let off: [u16; 7] = array::from_fn(|i| (i + 1) as u16);
		let res = off.map(|o| {
			if o != 7 {
				self.command.add(o).read_byte()
			} else {
				self.read_status()
			}
		});

		unsafe { transmute(res) }
	}

	pub fn self_diagnosis(&self) -> AtaOutput {
		self.write_lba28(LBA28::new(0));
		self.write_sector_count(0);
		self.write_command(Command::ExcuteDeviceDiagnostic);
		self.output()
	}

	fn pio_read_data(&self) -> u16 {
		let mut status = self.read_status();
		while status & Self::SIG_BUSY != 0 || status & Self::SIG_DRQ == 0 {
			status = self.read_status();
		}
		self.command.add(Self::DATA).read_u16()
	}

	/// Perform READ SECTORS command (PIO)
	pub fn read_sectors(&self, lba: LBA28, buf: &mut [RawSector]) {
		self.write_sector_count(buf.len() as u8);
		self.write_lba28(lba);
		self.write_command(Command::ReadSectors);

		for sector in buf {
			for word in &mut sector.0 {
				*word = self.pio_read_data();
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

	#[inline(always)]
	/// This function reads `Alternate Status Register` to avoid that the interrupt pending bit is cleard.
	fn read_status(&self) -> u8 {
		self.control.read_byte()
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
		write!(f, "err: 0b{:08b}\n", self.error)?;
		write!(f, "dev: {}\n", dev)?;
		write!(f, "lba: 0x{:x}\n", self.lba())?;
		write!(f, "sector count: 0x{:x}\n", self.sector_count)?;
		write!(f, "status: 0b{:08b}\n", self.status)?;

		Ok(())
	}
}

#[repr(u8)]
pub enum Command {
	ReadDMA = 0xc8,
	WriteDMA = 0xca,
	ReadSectors = 0x20,
	IdentifyDevice = 0xec,
	ExcuteDeviceDiagnostic = 0x90,
	FlushCache = 0xe7, // ?
}

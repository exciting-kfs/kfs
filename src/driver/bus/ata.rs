use core::{
	array,
	fmt::Display,
	mem::{transmute, MaybeUninit},
	ops::Deref,
};

use alloc::{string::String, vec::Vec};

use crate::{
	driver::ide::{dev_num::DevNum, lba::LBA28},
	io::pmio::Port,
	util::bitrange::{BitData, BitRange},
};

pub struct AtaController {
	command: Port,
	control: Port,
	is_2nd_dev: bool,
	intr_pending: bool,
}

#[repr(align(512))]
pub struct RawSector([u16; 256]);

impl RawSector {
	pub const fn empty() -> Self {
		Self([0; 256])
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
	const SIG_DRDY: u8 = 0x40;
	const SIG_DRQ: u8 = 0x08;

	const N_IEN: u8 = 1;
	const DEVICE_BIT: u8 = 4;

	pub const fn new(command_base: u16, control_base: u16) -> Self {
		Self {
			command: Port::new(command_base),
			control: Port::new(control_base),
			is_2nd_dev: false,
			intr_pending: false,
		}
	}

	pub fn set_interrupt(&self, on: bool) {
		self.control.write_byte((!on as u8) << Self::N_IEN);
	}

	pub fn set_device(&mut self, dev_num: DevNum) {
		self.is_2nd_dev = dev_num.index_in_channel() == 1;
	}

	pub fn interrupt_pending(&self) -> bool {
		self.intr_pending
	}

	pub fn interrupt_resolve(&mut self) {
		self.intr_pending = false;
	}

	pub fn write_dma(&mut self, lba: LBA28, sector_count: u16) {
		self.do_command(Command::WriteDMA, lba, sector_count);
		self.intr_pending = true;
	}

	pub fn read_dma(&mut self, lba: LBA28, sector_count: u16) {
		self.do_command(Command::ReadDMA, lba, sector_count);
		self.intr_pending = true;
	}

	// HI0 ~ HI4
	pub fn do_command(&self, command: Command, lba: LBA28, sector_count: u16) {
		self.device_select();

		self.write_lba28(lba);
		self.write_sector_count(sector_count);
		self.write_command(command);
	}

	pub fn write_command(&self, command: Command) {
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
		let select = (self.is_2nd_dev as u8) << Self::DEVICE_BIT;
		self.command.add(Self::DEVICE).write_byte(select);
		self.wait(|status| !Self::is_busy(status) && !Self::is_drq(status));

		self.write_lba28(LBA28::new(0));
		self.write_sector_count(0);
		self.write_command(Command::ExcuteDeviceDiagnostic);

		self.output()
	}

	/// Perform READ SECTORS command (PIO)
	///
	/// - Don't use at nIEN == 0.
	pub fn read_sectors(&self, lba: LBA28, buf: &mut [MaybeUninit<RawSector>]) {
		self.do_command(Command::ReadSectors, lba, buf.len() as u16);
		self.wait(|status| Self::is_drq(status));

		for sector in buf {
			let (chunks, _) = sector.as_bytes_mut().as_chunks_mut::<2>();
			for word in chunks {
				*word = unsafe { transmute(self.pio_read_data()) };
			}
		}
	}

	fn pio_read_data(&self) -> u16 {
		self.wait(|status| !Self::is_busy(status) || Self::is_drq(status));
		self.command.add(Self::DATA).read_u16()
	}

	pub fn identify_device(&self) -> AtaId {
		self.do_command(Command::IdentifyDevice, LBA28::new(0), 0);

		let mut data = RawSector([0; 256]);
		for word in &mut data.0 {
			*word = self.command.add(Self::DATA).read_u16();
		}

		AtaId { data }
	}

	pub fn is_idle(&self) -> bool {
		let status = self.read_status();
		!Self::is_busy(status) && !Self::is_drq(status)
	}

	#[inline(always)]
	/// This function reads `Alternate Status Register` to avoid that the interrupt pending bit is cleard.
	fn read_status(&self) -> u8 {
		self.control.read_byte()
	}

	#[inline(always)]
	fn is_busy(status: u8) -> bool {
		status & Self::SIG_BUSY > 0
	}

	#[inline(always)]
	fn is_drq(status: u8) -> bool {
		status & Self::SIG_DRQ > 0
	}

	#[inline(always)]
	fn is_drdy(status: u8) -> bool {
		status & Self::SIG_DRDY > 0
	}

	fn wait_400ns(&self) {
		for _ in 0..4 {
			let _ = self.control.read_byte();
		}
	}

	fn wait<F: Fn(u8) -> bool>(&self, condition: F) {
		let mut status = self.read_status();
		while !condition(status) {
			status = self.read_status();
		}
	}

	fn device_select(&self) {
		let select = (self.is_2nd_dev as u8) << Self::DEVICE_BIT;
		self.command.add(Self::DEVICE).write_byte(select);
		self.wait(|status| {
			!Self::is_busy(status) && !Self::is_drq(status) && Self::is_drdy(status)
		});
	}

	fn write_lba28(&self, lba: LBA28) {
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
				| ((self.is_2nd_dev as u8) << Self::DEVICE_BIT)
				| (1 << 6),
		);
	}

	fn write_sector_count(&self, count: u16) {
		debug_assert!(count <= 256);
		let count = match count == 256 {
			true => 0 as u8,
			false => count as u8,
		};
		self.command.add(Self::SECTOR_COUNT).write_byte(count);
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

#[derive(PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Command {
	ReadDMA = 0xc8,
	WriteDMA = 0xca,
	ReadSectors = 0x20,
	IdentifyDevice = 0xec,
	ExcuteDeviceDiagnostic = 0x90,
	FlushCache = 0xe7, // ?
}

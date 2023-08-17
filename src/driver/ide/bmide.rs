use core::{fmt::Display, mem::MaybeUninit};

use crate::{io::pmio::Port, sync::locked::Locked};

/// BMIDE[channel]
pub static BMIDE: [Locked<MaybeUninit<BMIDE>>; 2] = [Locked::uninit(), Locked::uninit()];

/// BUS MASTER IDE
pub struct BMIDE {
	base: Port,
	is_2nd_channel: bool,
}

impl BMIDE {
	const ERROR_BIT: u8 = 1 << 1;
	const INTERRUPT_BIT: u8 = 1 << 2;
	const STATUS_CLEAR: u8 = Self::ERROR_BIT | Self::INTERRUPT_BIT;

	const COMMAND: u16 = 0x0;
	const STATUS: u16 = 0x2;
	const PRDTR: u16 = 0x4;

	pub fn init(port: u16) {
		BMIDE[0].lock().write(BMIDE::new(port, false));
		BMIDE[1].lock().write(BMIDE::new(port + 0x08, true));
	}

	pub const fn new(base: u16, is_2nd_channel: bool) -> Self {
		Self {
			base: Port::new(base),
			is_2nd_channel,
		}
	}

	fn write_status(&self, data: u8) {
		self.base.add(Self::STATUS).write_byte(data);
	}

	fn write_command(&self, data: u8) {
		self.base.add(Self::COMMAND).write_byte(data);
	}

	fn read_command(&self) -> u8 {
		self.base.add(Self::COMMAND).read_byte()
	}

	fn read_status(&self) -> u8 {
		self.base.add(Self::STATUS).read_byte()
	}

	#[inline(always)]
	fn dma_init_status(&self) -> u8 {
		Self::STATUS_CLEAR | (1 << 5) << self.is_2nd_channel as u8
	}

	pub fn set_dma_read(&self) {
		self.write_command(1 << 3);
		self.write_status(self.dma_init_status());
	}

	pub fn set_dma_write(&self) {
		self.write_command(0);
		self.write_status(self.dma_init_status());
	}

	pub fn sync_data(&self) {
		self.read_status();
	}

	pub fn clear(&self) {
		self.write_status(Self::STATUS_CLEAR);
	}

	pub fn start(&self) {
		self.write_command(self.read_command() | 0x01);
	}

	pub fn stop(&self) {
		self.write_command(self.read_command() & 0xfe);
	}

	pub fn is_error(&self) -> bool {
		(self.read_status() & Self::ERROR_BIT) == Self::ERROR_BIT
	}

	pub fn register_prdt(&self, paddr: u32) {
		self.base.add(Self::PRDTR).write_u32(paddr);
	}
}

impl Display for BMIDE {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "[BusMasterIDE]\n")?;
		write!(f, "Command: {:x}\n", self.read_command())?;
		write!(f, "Status:  {:x}\n", self.read_status())?;
		write!(
			f,
			"PRD Table: {:x}\n",
			self.base.add(Self::PRDTR).read_u32()
		)?;

		Ok(())
	}
}

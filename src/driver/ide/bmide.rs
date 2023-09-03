use core::{fmt::Display, mem::MaybeUninit, ptr::addr_of};

use crate::{io::pmio::Port, mm::util::virt_to_phys, sync::locked::Locked};

use super::prd::PRD;

pub static BMIDE: [Locked<MaybeUninit<BMIDE>>; 2] = [Locked::uninit(), Locked::uninit()]; // channel

/// BUS MASTER IDE
pub struct BMIDE {
	base: Port,
	is_2nd_channel: bool,
	prd_table: [PRD; 128],
}

impl BMIDE {
	const ERROR_BIT: u8 = 1 << 1;
	const INTERRUPT_BIT: u8 = 1 << 2;
	const STATUS_CLEAR: u8 = Self::ERROR_BIT | Self::INTERRUPT_BIT;

	const COMMAND: u16 = 0x0;
	const STATUS: u16 = 0x2;
	const PRDTR: u16 = 0x4;

	pub fn init(port: u16) {
		let mut bmide0 = BMIDE[0].lock();
		let mut bmide1 = BMIDE[1].lock();

		bmide0.write(BMIDE::new(port, false));
		bmide1.write(BMIDE::new(port + 0x08, true));

		unsafe {
			bmide0.assume_init_ref().load_prd_table();
			bmide1.assume_init_ref().load_prd_table();
		}
	}

	pub const fn new(base: u16, is_2nd_channel: bool) -> Self {
		Self {
			base: Port::new(base),
			is_2nd_channel,
			prd_table: [PRD::new(0, 0); 128],
		}
	}

	pub fn prd_table(&mut self) -> &mut [PRD] {
		&mut self.prd_table
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

	pub fn load_prd_table(&self) {
		let paddr = virt_to_phys(addr_of!(self.prd_table) as usize);
		self.base.add(Self::PRDTR).write_u32(paddr as u32);
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

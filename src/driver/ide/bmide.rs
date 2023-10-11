use core::{fmt::Display, ptr::addr_of};

use alloc::vec::Vec;

use crate::{
	driver::bus::pci::{self, find_device, header::HeaderType0},
	io::pmio::Port,
	mm::util::virt_to_phys,
};

use super::{
	block::Block,
	dma::{hook::ItemWB, DmaOps},
	prd::PRD,
	IDE_CLASS_CODE,
};

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

	pub fn for_each_channel() -> Result<[BMIDE; 2], pci::Error> {
		// PCI CONFIGURATION SPACE
		let bdf = find_device(IDE_CLASS_CODE)?;
		let h0 = HeaderType0::get(&bdf)?;
		bdf.set_busmaster(true);

		// BUS MASTER IDE
		let port = match h0.bar4 & 0x1 == 0x1 {
			true => h0.bar4 & 0xffff_fffc,
			false => h0.bar4 & 0xffff_fff0,
		} as u16;

		Ok([BMIDE::new(port, false), BMIDE::new(port + 0x08, true)])
	}

	const fn new(base: u16, is_2nd_channel: bool) -> Self {
		Self {
			base: Port::new(base),
			is_2nd_channel,
			prd_table: [PRD::new(0, 0); 128],
		}
	}

	pub fn prd_table(&mut self) -> &mut [PRD] {
		&mut self.prd_table
	}

	pub fn set_prd_table(&mut self, blocks: &Vec<Block>) {
		let prdt = &mut self.prd_table;

		// set BMIDE
		for (i, block) in blocks.iter().enumerate() {
			prdt[i] = PRD::new(block.as_phys_addr(), block.size() as u16);
		}
		prdt[blocks.len() - 1].set_eot(true);
	}

	pub fn set_prd_table_wb(&mut self, blocks: &Vec<ItemWB>) {
		let prdt = &mut self.prd_table;

		// set BMIDE
		for (i, item) in blocks.iter().enumerate() {
			prdt[i] = PRD::new(item.as_phys_addr(), item.size() as u16);
		}
		prdt[blocks.len() - 1].set_eot(true);
	}

	fn write_status(&mut self, data: u8) {
		self.base.add(Self::STATUS).write_byte(data);
	}

	fn write_command(&mut self, data: u8) {
		self.base.add(Self::COMMAND).write_byte(data);
	}

	fn read_command(&self) -> u8 {
		self.base.add(Self::COMMAND).read_byte()
	}

	fn read_status(&self) -> u8 {
		self.base.add(Self::STATUS).read_byte()
	}

	#[inline]
	fn dma_init_status(&self) -> u8 {
		Self::STATUS_CLEAR | (1 << 5) << self.is_2nd_channel as u8
	}

	pub fn set_dma(&mut self, ops: DmaOps) {
		let command = match ops {
			DmaOps::Read => 1 << 3,
			DmaOps::Write => 0,
		};

		self.write_command(command);
		self.write_status(self.dma_init_status());
	}

	pub fn sync_data(&self) {
		self.read_status();
	}

	pub fn clear(&mut self) {
		self.write_status(Self::STATUS_CLEAR);
	}

	pub fn start(&mut self) {
		self.write_command(self.read_command() | 0x01);
	}

	pub fn stop(&mut self) {
		self.write_command(self.read_command() & 0xfe);
	}

	pub fn is_error(&self) -> bool {
		(self.read_status() & Self::ERROR_BIT) == Self::ERROR_BIT
	}

	pub fn load_prd_table(&mut self) {
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

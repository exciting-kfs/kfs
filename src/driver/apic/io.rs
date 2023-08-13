use core::mem::MaybeUninit;
use core::ptr::{addr_of_mut, NonNull};

use crate::acpi::IOAPIC_INFO;
use crate::io::pmio::Port;
use crate::mm::constant::HIGH_IO_OFFSET;
use crate::sync::locked::Locked;
use crate::util::bitrange::{BitData, BitRange};

pub const KEYBOARD_IRQ: usize = 1;
pub const SERIAL_COM1_IRQ: usize = 4;
pub const SERIAL_COM2_IRQ: usize = 3;

pub static IO_APIC: Locked<MaybeUninit<IOAPIC>> = Locked::uninit();

#[derive(Debug)]
pub enum IOAPICError {
	UnknownLayout,
	InvalidBaseAddr,
	IndexOutOfRange,
}

pub fn init() -> Result<(), IOAPICError> {
	// assume there is only one I/O APIC.
	if IOAPIC_INFO.io_apics.len() != 1 {
		return Err(IOAPICError::UnknownLayout);
	}

	let mmio_base = IOAPIC_INFO.io_apics[0].address as usize;

	// MMIO address for I/O APIC must be reside in HIGH_IO
	if mmio_base < HIGH_IO_OFFSET {
		return Err(IOAPICError::InvalidBaseAddr);
	}

	let mut apic = IO_APIC.lock();

	apic.write(IOAPIC::new(unsafe {
		NonNull::new_unchecked(mmio_base as *mut DirectRegister)
	}));

	let apic = unsafe { apic.assume_init_mut() };

	let mut keyboard_redir = apic.read_redir(KEYBOARD_IRQ)?;
	let mut serial_com1 = apic.read_redir(SERIAL_COM1_IRQ)?;

	keyboard_redir.set_default(0x21);
	serial_com1.set_default(0x23);

	apic.write_redir(KEYBOARD_IRQ, keyboard_redir)?;
	apic.write_redir(SERIAL_COM1_IRQ, serial_com1)?;

	disable_8259_pic();

	Ok(())
}

fn disable_8259_pic() {
	let master_data: Port = Port::new(0x21);
	let slave_data: Port = Port::new(0xa1);

	master_data.write_byte(0xff);
	slave_data.write_byte(0xff);
}

pub struct IOAPIC {
	direct_reg: NonNull<DirectRegister>,
}

impl IOAPIC {
	const ID_INDEX: usize = 0x0;
	const VERSION_INDEX: usize = 0x1;
	const REDIR_TABLE_BASE_INDEX: usize = 0x10;
	const REDIR_TABLE_COUNT: usize = 24;

	fn new(direct_reg: NonNull<DirectRegister>) -> Self {
		Self { direct_reg }
	}

	fn reg_ptr(&self) -> *mut DirectRegister {
		self.direct_reg.as_ptr()
	}

	unsafe fn select_indirect_reg(&mut self, index: usize) {
		addr_of_mut!((*self.reg_ptr()).index).write_volatile(index as u8);
	}

	unsafe fn read_indirect_reg(&mut self, index: usize) -> u32 {
		self.select_indirect_reg(index);
		addr_of_mut!((*self.reg_ptr()).data).read_volatile()
	}

	unsafe fn write_indirect_reg(&mut self, index: usize, data: u32) {
		self.select_indirect_reg(index);
		addr_of_mut!((*self.reg_ptr()).data).write_volatile(data);
	}

	fn redir_table_index(index: usize) -> (usize, usize) {
		let low_idx = Self::REDIR_TABLE_BASE_INDEX + index * 2;

		(low_idx, low_idx + 1)
	}

	pub fn end_of_interrupt(&mut self) {
		unsafe { addr_of_mut!((*self.reg_ptr()).eoi).write_volatile(0) };
	}

	// TODO: use BitData
	pub fn id(&mut self) -> u32 {
		unsafe { self.read_indirect_reg(Self::ID_INDEX) }
	}

	// TODO: use BitData
	pub fn version(&mut self) -> u32 {
		unsafe { self.read_indirect_reg(Self::VERSION_INDEX) }
	}

	pub fn write_redir(
		&mut self,
		irq_number: usize,
		entry: RedirectionTable,
	) -> Result<(), IOAPICError> {
		if irq_number >= Self::REDIR_TABLE_COUNT {
			return Err(IOAPICError::IndexOutOfRange);
		}

		let (low_idx, high_idx) = Self::redir_table_index(irq_number);

		unsafe {
			self.write_indirect_reg(low_idx, entry.low.get_raw_bits() as u32);
			self.write_indirect_reg(high_idx, entry.high.get_raw_bits() as u32);
		};

		Ok(())
	}

	pub fn read_redir(&mut self, irq_number: usize) -> Result<RedirectionTable, IOAPICError> {
		if irq_number >= Self::REDIR_TABLE_COUNT {
			return Err(IOAPICError::IndexOutOfRange);
		}

		let (low_idx, high_idx) = Self::redir_table_index(irq_number);

		let low;
		let high;
		unsafe {
			low = self.read_indirect_reg(low_idx);
			high = self.read_indirect_reg(high_idx);
		};

		Ok(RedirectionTable {
			low: BitData::new(low as usize),
			high: BitData::new(high as usize),
		})
	}
}

pub fn set_irq_mask(irq_number: usize, mask: bool) -> Result<(), IOAPICError> {
	let mut lock = IO_APIC.lock();
	let ioapic = unsafe { lock.assume_init_mut() };

	let mut irq = ioapic.read_redir(irq_number)?;
	irq.set_mask(mask);
	ioapic.write_redir(irq_number, irq)?;
	Ok(())
}

#[repr(packed)]
struct DirectRegister {
	// offset = 0
	index: u8,
	_pad1: [u8; 15],

	// offset = 0x10
	data: u32,
	_pad2: [u8; 44],

	// offset = 0x40
	eoi: u32,
}

pub struct RedirectionTable {
	low: BitData,
	high: BitData,
}

impl RedirectionTable {
	// for self.low
	const VECTOR: BitRange = BitRange::new(0, 8);
	const DELIVERY_MODE: BitRange = BitRange::new(8, 11);
	const DEST_MODE: BitRange = BitRange::new(11, 12);
	const DELIVERY_STATUS: BitRange = BitRange::new(12, 13);
	const POLARITY: BitRange = BitRange::new(13, 14);
	const REMOTE_IRR: BitRange = BitRange::new(14, 15);
	const TRIGGER_MODE: BitRange = BitRange::new(15, 16);
	const MASK: BitRange = BitRange::new(16, 17);
	const RESERVED_LOW: BitRange = BitRange::new(17, 32);

	// for self.high
	const RESERVED_HIGH: BitRange = BitRange::new(0, 16);
	const EXTENDED_DEST_ID: BitRange = BitRange::new(16, 24);
	const DESTINATION: BitRange = BitRange::new(24, 32);

	pub fn new(low: usize, high: usize) -> Self {
		Self {
			low: BitData::new(low),
			high: BitData::new(high),
		}
	}

	pub fn set_default(&mut self, vector: usize) -> &mut Self {
		self.set_vector(vector)
			.set_delivery_mode(DeliveryMode::Fixed)
			.set_dest_mode(DestMode::Physical)
			.set_trigger_mode(TriggerMode::Edge)
			.set_mask(false)
			.set_destination(0)
	}

	pub fn set_vector(&mut self, vector: usize) -> &mut Self {
		self.low
			.erase_bits(&Self::VECTOR)
			.shift_add_bits(&Self::VECTOR, vector);

		self
	}

	pub fn set_delivery_mode(&mut self, mode: DeliveryMode) -> &mut Self {
		self.low
			.erase_bits(&Self::DELIVERY_MODE)
			.shift_add_bits(&Self::DELIVERY_MODE, mode as usize);

		self
	}

	pub fn set_dest_mode(&mut self, mode: DestMode) -> &mut Self {
		self.low
			.erase_bits(&Self::DEST_MODE)
			.shift_add_bits(&Self::DEST_MODE, mode as usize);

		self
	}

	pub fn get_delivery_status(&self) -> DeliveryStatus {
		match self.low.get_bits(&Self::DELIVERY_STATUS) {
			0 => DeliveryStatus::Idle,
			1 => DeliveryStatus::Pending,
			_ => panic!("unknown delivery status"),
		}
	}

	pub fn set_polarity(&mut self, polarity: Polarity) -> &mut Self {
		self.low
			.erase_bits(&Self::POLARITY)
			.shift_add_bits(&Self::POLARITY, polarity as usize);

		self
	}

	// TODO: pub fn (get/set)_remote_irr

	pub fn set_trigger_mode(&mut self, mode: TriggerMode) -> &mut Self {
		self.low
			.erase_bits(&Self::TRIGGER_MODE)
			.shift_add_bits(&Self::TRIGGER_MODE, mode as usize);

		self
	}

	pub fn set_mask(&mut self, mask: bool) -> &mut Self {
		self.low
			.erase_bits(&Self::MASK)
			.shift_add_bits(&Self::MASK, mask as usize);

		self
	}

	pub fn get_extended_dest_id(&self) -> usize {
		self.high.shift_get_bits(&Self::EXTENDED_DEST_ID)
	}

	pub fn set_destination(&mut self, dest: usize) -> &mut Self {
		self.high
			.erase_bits(&Self::DESTINATION)
			.shift_add_bits(&Self::DESTINATION, dest);

		self
	}
}

pub enum TriggerMode {
	Edge = 0,
	Level = 1,
}

pub enum Polarity {
	ActiveHigh = 0,
	ActiveLow = 1,
}

pub enum DeliveryStatus {
	Idle = 0,
	Pending = 1,
}

pub enum DestMode {
	Physical = 0,
	Logical = 1,
}

pub enum DeliveryMode {
	Fixed = 0b000,
	LowerstPriority = 0b001,
	SMI = 0b010,
	NMI = 0b100,
	INIT = 0b101,
	ExtINT = 0b111,
}

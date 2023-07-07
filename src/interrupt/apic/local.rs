use core::{
	cell::UnsafeCell,
	ptr::{addr_of_mut, NonNull},
};

use crate::config::TIMER_FREQUENCY_HZ;
use crate::mm::{
	alloc::virt::AddressSpace,
	constant::{MB, PAGE_MASK},
};
use crate::pr_info;
use crate::util::{
	arch::cpuid::CPUID,
	bitrange::{BitData, BitRange},
};

use super::MSR_APIC_BASE;
use macros::lapic_register;

pub static LOCAL_APIC: LocalAPIC = LocalAPIC::uninit();

pub struct LocalAPIC {
	register: UnsafeCell<NonNull<Register>>,
}

impl LocalAPIC {
	const fn uninit() -> Self {
		Self {
			register: UnsafeCell::new(NonNull::dangling()),
		}
	}

	unsafe fn init(&self, ptr: NonNull<Register>) {
		self.register.get().write(ptr);
	}

	fn reg_ptr(&self) -> *mut Register {
		unsafe { (*self.register.get()).as_ptr() }
	}

	pub fn id(&self) -> usize {
		let raw_id = unsafe { lapic_register!(id).read_volatile() } as usize;

		raw_id >> 24
	}

	// TODO: separate VERSION / LVT entries
	pub fn version(&self) -> usize {
		unsafe { lapic_register!(version).read_volatile() as usize }
	}

	pub fn read_timer(&self) -> Timer {
		unsafe { lapic_register!(timer).read_volatile() }
	}

	pub fn write_timer(&self, timer: Timer) {
		unsafe { lapic_register!(timer).write_volatile(timer) };
	}

	pub fn set_timer_divider(&self, divider: TimerDivider) {
		unsafe {
			let divider_ptr = lapic_register!(divide_configuration);
			let original = divider_ptr.read_volatile();

			divider_ptr.write_volatile(original | divider as u32);
		}
	}

	pub fn write_timer_initial_count(&self, count: usize) {
		unsafe { lapic_register!(initial_count).write_volatile(count as u32) };
	}

	pub fn read_timer_current_count(&self) -> usize {
		unsafe { lapic_register!(current_count).read_volatile() as usize }
	}

	pub fn end_of_interrupt(&self) {
		unsafe { lapic_register!(end_of_interrupt).write_volatile(0) }
		// unsafe { addr_of_mut!((*self.reg_ptr()).end_of_interrupt).write_volatile(0) }
	}
}

#[macro_use]
mod macros {
	macro_rules! lapic_register {
		($id: ident) => {
			addr_of_mut!((*LOCAL_APIC.reg_ptr()).$id)
		};
	}

	pub(super) use lapic_register;
}

// safety: Local APIC always located at Per-CPU I/O memory.
unsafe impl Sync for LocalAPIC {}

#[repr(packed)]
struct Register {
	_pad1: [u8; 32],
	id: u32,
	_pad2: [u8; 12],
	version: u32,
	_pad3: [u8; 76],
	task_priority: u32,
	_pad4: [u8; 12],
	arbitration_priority: u32,
	_pad5: [u8; 12],
	processor_priority: u32,
	_pad6: [u8; 12],
	end_of_interrupt: u32,
	_pad7: [u8; 12],
	remote_read: u32,
	_pad8: [u8; 12],
	logical_destination: u32,
	_pad9: [u8; 12],
	destination_format: u32,
	_pad10: [u8; 12],
	spurious_interrupt_vector: u32,
	_pad11: [u8; 12],
	in_service0: u32,
	_pad12: [u8; 12],
	in_service1: u32,
	_pad13: [u8; 12],
	in_service2: u32,
	_pad14: [u8; 12],
	in_service3: u32,
	_pad15: [u8; 12],
	in_service4: u32,
	_pad16: [u8; 12],
	in_service5: u32,
	_pad17: [u8; 12],
	in_service6: u32,
	_pad18: [u8; 12],
	in_service7: u32,
	_pad19: [u8; 12],
	trigger_mode0: u32,
	_pad20: [u8; 12],
	trigger_mode1: u32,
	_pad21: [u8; 12],
	trigger_mode2: u32,
	_pad22: [u8; 12],
	trigger_mode3: u32,
	_pad23: [u8; 12],
	trigger_mode4: u32,
	_pad24: [u8; 12],
	trigger_mode5: u32,
	_pad25: [u8; 12],
	trigger_mode6: u32,
	_pad26: [u8; 12],
	trigger_mode7: u32,
	_pad27: [u8; 12],
	interrupt_request0: u32,
	_pad28: [u8; 12],
	interrupt_request1: u32,
	_pad29: [u8; 12],
	interrupt_request2: u32,
	_pad30: [u8; 12],
	interrupt_request3: u32,
	_pad31: [u8; 12],
	interrupt_request4: u32,
	_pad32: [u8; 12],
	interrupt_request5: u32,
	_pad33: [u8; 12],
	interrupt_request6: u32,
	_pad34: [u8; 12],
	interrupt_request7: u32,
	_pad35: [u8; 12],
	error_status: u32,
	_pad36: [u8; 108],
	corrected_machine_check_interrupt: u32,
	_pad37: [u8; 12],
	interrupt_command0: u32,
	_pad38: [u8; 12],
	interrupt_command1: u32,
	_pad39: [u8; 12],
	timer: Timer,
	_pad40: [u8; 12],
	thermal_sensor: u32,
	_pad41: [u8; 12],
	performance_monitoring_counters: u32,
	_pad42: [u8; 12],
	lint0: u32,
	_pad43: [u8; 12],
	lint1: u32,
	_pad44: [u8; 12],
	error: u32,
	_pad45: [u8; 12],
	initial_count: u32,
	_pad46: [u8; 12],
	current_count: u32,
	_pad47: [u8; 76],
	// offset must be 0x3e0
	divide_configuration: u32,
}

#[repr(transparent)]
pub struct Timer {
	data: BitData,
}

pub enum TimerMode {
	OneShot = 0b00,
	Periodic = 0b01,
	TSCDeadline = 0b10,
}

pub enum DeliveryStatus {
	Idle = 0,
	SendPending = 1,
}

pub enum TimerDivider {
	By1 = 0b1011,
	By2 = 0b0000,
	By4 = 0b0001,
	By8 = 0b0010,
	By16 = 0b0011,
	By32 = 0b1000,
	By64 = 0b1001,
	By128 = 0b1010,
}

impl Timer {
	const VECTOR: BitRange = BitRange::new(0, 8);
	const DELIVERY_STATUS: BitRange = BitRange::new(12, 13);
	const MASK: BitRange = BitRange::new(16, 17);
	const TIMER_MODE: BitRange = BitRange::new(17, 19);

	pub fn set_mask(&mut self, mask: bool) -> &mut Self {
		self.data
			.erase_bits(&Self::MASK)
			.shift_add_bits(&Self::MASK, mask as usize);

		self
	}

	pub fn set_vector(&mut self, vector: usize) -> &mut Self {
		self.data
			.erase_bits(&Self::VECTOR)
			.shift_add_bits(&Self::VECTOR, vector);

		self
	}

	pub fn set_timer_mode(&mut self, mode: TimerMode) -> &mut Self {
		self.data
			.erase_bits(&Self::TIMER_MODE)
			.shift_add_bits(&Self::TIMER_MODE, mode as usize);

		self
	}

	pub fn get_delivery_status(&self) -> DeliveryStatus {
		let status = self.data.get_bits(&Self::DELIVERY_STATUS);

		match status {
			0 => DeliveryStatus::Idle,
			1 => DeliveryStatus::SendPending,
			_ => panic!("unknown delivery status"),
		}
	}
}

#[derive(Debug)]
pub enum LocalAPICError {
	InvalidBaseAddr,
}

pub fn init() -> Result<(), LocalAPICError> {
	let base = MSR_APIC_BASE.read().low & PAGE_MASK;

	// base must be in HighIO region
	match AddressSpace::identify(base) {
		AddressSpace::HighIO => (),
		_ => return Err(LocalAPICError::InvalidBaseAddr),
	}

	unsafe { LOCAL_APIC.init(NonNull::new_unchecked(base as *mut Register)) };

	init_timer();

	Ok(())
}

/// configure and start timer interrupt
/// TODO: acpi timer celibration or TSC deadline mode?
fn init_timer() {
	let cpuid = CPUID::run(0x16, 0);
	let bus_freq = cpuid.ecx * MB;
	let count = bus_freq / TIMER_FREQUENCY_HZ;

	pr_info!("local APIC: bus freqeuncy(MHz): {:?}", cpuid.ecx);
	pr_info!("local APIC: timer freqeuncy(Hz): {:?}", TIMER_FREQUENCY_HZ);

	let mut timer = LOCAL_APIC.read_timer();

	timer
		.set_mask(false)
		.set_timer_mode(TimerMode::Periodic)
		.set_vector(0x22);

	LOCAL_APIC.write_timer(timer);
	LOCAL_APIC.set_timer_divider(TimerDivider::By1);
	LOCAL_APIC.write_timer_initial_count(count);
}

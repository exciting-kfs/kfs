pub mod ipi;
pub mod timer;

use core::ptr;

use crate::{interrupt::apic::local::timer::Mode, mm::util::phys_to_virt};

pub static mut PBASE: usize = 0;

#[repr(usize)]
#[derive(Clone, Copy)]
pub enum Register {
	ID = 0x20,
	Version = 0x30,
	TaskPriorty = 0x80,
	ArbitrationPriority = 0x90,
	ProcessorPriority = 0xa0,
	EndOfInterrupt = 0xb0,
	RemoteRead = 0xc0,
	LogicalDestination = 0xd0,
	DestinationFormat = 0xe0,
	SpuriousInterruptVector = 0xf0,
	InService0 = 0x100,
	InService1 = 0x110,
	InService2 = 0x120,
	InService3 = 0x130,
	InService4 = 0x140,
	InService5 = 0x150,
	InService6 = 0x160,
	InService7 = 0x170,
	TriggerMode0 = 0x180,
	TriggerMode1 = 0x190,
	TriggerMode2 = 0x1a0,
	TriggerMode3 = 0x1b0,
	TriggerMode4 = 0x1c0,
	TriggerMode5 = 0x1d0,
	TriggerMode6 = 0x1e0,
	TriggerMode7 = 0x1f0,
	InterruptRequest0 = 0x200,
	InterruptRequest1 = 0x210,
	InterruptRequest2 = 0x220,
	InterruptRequest3 = 0x230,
	InterruptRequest4 = 0x240,
	InterruptRequest5 = 0x250,
	InterruptRequest6 = 0x260,
	InterruptRequest7 = 0x270,
	ErrorStatus = 0x280,
	CorrectedMachineCheckInterrupt = 0x2f0,
	InterruptCommand0 = 0x300,
	InterruptCommand1 = 0x310,
	LvtTimer = 0x320,
	LvtThermalSensor = 0x330,
	LvtPerformaceMonitoringCounters = 0x340,
	LvtLint0 = 0x350,
	LvtLint1 = 0x360,
	LvtError = 0x370,
	InitialCount = 0x380,
	CurrentCount = 0x390,
	DivideConfiguration = 0x3e0,
}

impl Register {
	pub fn addr(&self) -> usize {
		vbase() + *self as usize
	}

	pub fn read(&self) -> u32 {
		let ptr = self.addr() as *const u32;
		unsafe { ptr::read_volatile(ptr) }
	}

	pub fn write(&self, value: u32) {
		let ptr = self.addr() as *mut u32;
		unsafe { ptr::write_volatile(ptr, value) };
	}

	pub fn iter() -> core::slice::Iter<'static, Register> {
		const REGISTERS: [Register; 44] = [
			Register::ID,
			Register::Version,
			Register::TaskPriorty,
			Register::ArbitrationPriority,
			Register::ProcessorPriority,
			// Register::RemoteRead,
			Register::LogicalDestination,
			Register::DestinationFormat,
			Register::SpuriousInterruptVector,
			Register::InService0,
			Register::InService1,
			Register::InService2,
			Register::InService3,
			Register::InService4,
			Register::InService5,
			Register::InService6,
			Register::InService7,
			Register::TriggerMode0,
			Register::TriggerMode1,
			Register::TriggerMode2,
			Register::TriggerMode3,
			Register::TriggerMode4,
			Register::TriggerMode5,
			Register::TriggerMode6,
			Register::TriggerMode7,
			Register::InterruptRequest0,
			Register::InterruptRequest1,
			Register::InterruptRequest2,
			Register::InterruptRequest3,
			Register::InterruptRequest4,
			Register::InterruptRequest5,
			Register::InterruptRequest6,
			Register::InterruptRequest7,
			Register::ErrorStatus,
			// Register::CorrectedMachineCheckInterrupt,
			Register::InterruptCommand0,
			Register::InterruptCommand1,
			Register::LvtTimer,
			Register::LvtThermalSensor,
			Register::LvtPerformaceMonitoringCounters,
			Register::LvtLint0,
			Register::LvtLint1,
			Register::LvtError,
			Register::InitialCount,
			Register::CurrentCount,
			Register::DivideConfiguration,
		];
		REGISTERS.iter()
	}
}

impl core::fmt::Display for Register {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let s = match self {
			Self::ArbitrationPriority => "APR",
			Self::CorrectedMachineCheckInterrupt => "CMCI",
			Self::CurrentCount => "CurrentCount",
			Self::DestinationFormat => "DFR",
			Self::DivideConfiguration => "DCR",
			Self::EndOfInterrupt => "EOI",
			Self::ErrorStatus => "Error",
			Self::ID => "ID",
			Self::InService0 => "ISR0",
			Self::InService1 => "ISR1",
			Self::InService2 => "ISR2",
			Self::InService3 => "ISR3",
			Self::InService4 => "ISR4",
			Self::InService5 => "ISR5",
			Self::InService6 => "ISR6",
			Self::InService7 => "ISR7",
			Self::InitialCount => "InitialCount",
			Self::InterruptCommand0 => "ICR0",
			Self::InterruptCommand1 => "ICR1",
			Self::InterruptRequest0 => "IRR0",
			Self::InterruptRequest1 => "IRR1",
			Self::InterruptRequest2 => "IRR2",
			Self::InterruptRequest3 => "IRR3",
			Self::InterruptRequest4 => "IRR4",
			Self::InterruptRequest5 => "IRR5",
			Self::InterruptRequest6 => "IRR6",
			Self::InterruptRequest7 => "IRR7",
			Self::LogicalDestination => "LDR",
			Self::LvtError => "LVT Error",
			Self::LvtLint0 => "LVT LINT0",
			Self::LvtLint1 => "LVT LINT1",
			Self::LvtPerformaceMonitoringCounters => "LVT Performance Monitoring Counters",
			Self::LvtThermalSensor => "LVT Thermal Sensor",
			Self::LvtTimer => "LVT Timer",
			Self::ProcessorPriority => "PPR",
			Self::RemoteRead => "RRD",
			Self::SpuriousInterruptVector => "Spurious Interrupt Vector",
			Self::TaskPriorty => "TPR",
			Self::TriggerMode0 => "TMR0",
			Self::TriggerMode1 => "TMR1",
			Self::TriggerMode2 => "TMR2",
			Self::TriggerMode3 => "TMR3",
			Self::TriggerMode4 => "TMR4",
			Self::TriggerMode5 => "TMR5",
			Self::TriggerMode6 => "TMR6",
			Self::TriggerMode7 => "TMR7",
			Self::Version => "Version",
		};
		write!(f, "{}", s)
	}
}

fn read_n<const N: usize>(init_register: Register, buf: &mut [u32; N]) {
	let addr = init_register.addr();
	for (i, b) in buf.iter_mut().enumerate() {
		let ptr = (addr + i * 0x10) as *mut u32;
		*b = unsafe { ptr::read_volatile(ptr) }
	}
}

pub fn read_in_service(buf: &mut [u32; 8]) {
	read_n(Register::InService0, buf)
}

pub fn read_interrupt_request(buf: &mut [u32; 8]) {
	read_n(Register::InterruptRequest0, buf)
}

pub fn read_interrupt_command(buf: &mut [u32; 2]) {
	read_n(Register::InterruptCommand0, buf)
}

pub fn write_interrupt_command(value: &[u32; 2]) {
	let addr = Register::InterruptCommand1.addr();
	value.iter().rev().enumerate().for_each(|(i, v)| {
		let ptr = (addr - i * 0x10) as *mut u32;
		unsafe { ptr.write(*v) };
	});
}

/// Caution
///
/// - Don't set different base address of each other cpu.
pub fn pbase() -> usize {
	unsafe { PBASE }
}

pub fn vbase() -> usize {
	phys_to_virt(pbase())
}

pub fn init() {
	// timer initialization
	const TIMER_FREQ_HZ: usize = 1000 * 1000; // TODO config? precision?
	timer::init(TIMER_FREQ_HZ, Mode::Periodic, 0x22);

	// disable LINT0, LINT1
	let v = Register::LvtLint0.read();
	Register::LvtLint0.write(v | (1 << 16));
	let v = Register::LvtLint1.read();
	Register::LvtLint1.write(v | (1 << 16));

	// enable LVT Error handler.
	let v = 0xfe;
	Register::LvtError.write(v);
}

pub fn end_of_interrupt() {
	Register::EndOfInterrupt.write(0);
}

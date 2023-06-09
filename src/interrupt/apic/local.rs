use core::ptr;

use crate::{
	mm::{constant::MB, util::phys_to_virt},
	pr_info,
	util::arch::cpuid::CPUID,
};

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
	TriggerMode0 = 0x180,
	TriggerMode1 = 0x190,
	TriggerMode2 = 0x1a0,
	TriggerMode3 = 0x1b0,
	InterruptRequest0 = 0x200,
	InterruptRequest1 = 0x210,
	InterruptRequest2 = 0x220,
	InterruptRequest3 = 0x230,
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

	pub fn read(&self) -> usize {
		let ptr = self.addr() as *const usize;
		unsafe { ptr::read_volatile(ptr) }
	}

	pub fn write(&self, value: usize) {
		let ptr = self.addr() as *mut usize;
		unsafe { ptr::write_volatile(ptr, value) };
	}

	pub fn iter() -> core::slice::Iter<'static, Register> {
		const REGISTERS: [Register; 35] = [
			Register::ID,
			Register::Version,
			Register::TaskPriorty,
			Register::ArbitrationPriority,
			Register::ProcessorPriority,
			Register::EndOfInterrupt,
			Register::RemoteRead,
			Register::LogicalDestination,
			Register::DestinationFormat,
			Register::SpuriousInterruptVector,
			Register::InService0,
			Register::InService1,
			Register::InService2,
			Register::InService3,
			Register::TriggerMode0,
			Register::TriggerMode1,
			Register::TriggerMode2,
			Register::TriggerMode3,
			Register::InterruptRequest0,
			Register::InterruptRequest1,
			Register::InterruptRequest2,
			Register::InterruptRequest3,
			Register::ErrorStatus,
			Register::CorrectedMachineCheckInterrupt,
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
			Self::InitialCount => "InitialCount",
			Self::InterruptCommand0 => "ICR0",
			Self::InterruptCommand1 => "ICR1",
			Self::InterruptRequest0 => "IRR0",
			Self::InterruptRequest1 => "IRR1",
			Self::InterruptRequest2 => "IRR2",
			Self::InterruptRequest3 => "IRR3",
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
			Self::Version => "Version",
		};
		write!(f, "{}", s)
	}
}

fn read_n<const N: usize>(init_register: Register, buf: &mut [usize; N]) {
	let addr = init_register.addr();
	for (i, b) in buf.iter_mut().enumerate() {
		let ptr = (addr + i * 0x10) as *mut usize;
		*b = unsafe { ptr::read_volatile(ptr) }
	}
}

pub fn read_in_service(buf: &mut [usize; 4]) {
	read_n(Register::InService0, buf)
}

pub fn read_interrupt_request(buf: &mut [usize; 4]) {
	read_n(Register::InterruptRequest0, buf)
}

pub fn read_interrupt_command(buf: &mut [usize; 2]) {
	read_n(Register::InterruptCommand0, buf)
}

pub fn write_interrupt_command(value: &[usize; 2]) {
	let addr = Register::InterruptCommand0.addr();
	value.iter().enumerate().for_each(|(i, v)| {
		let ptr = (addr + i * 0x10) as *mut usize;
		unsafe { *ptr = *v };
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
	set_timer_frequency();

	let mut v = Register::LvtTimer.read();
	v = v & !(1 << 16); // enable timer
	v = v | 0x22; // set vector number.
	Register::LvtTimer.write(v);
}

fn set_timer_frequency() {
	const TIMER_FREQUENCY_HZ: usize = 1024; // TODO config?
	let cpuid = CPUID::run(0x16, 0);

	let bus_freq = cpuid.ecx * MB;
	let count = bus_freq / TIMER_FREQUENCY_HZ;
	let freq = bus_freq / count;

	pr_info!("Bus freqeuncy(MHz): {:?}", cpuid.ecx);
	pr_info!("Timer interrupt freqeuncy(Hz): {:?}", freq);

	let mut div_conf = Register::DivideConfiguration.read();
	div_conf = div_conf | 0b1011; // divided by 1 (bus_freq / n)
	Register::DivideConfiguration.write(div_conf);
	Register::InitialCount.write(count);
}

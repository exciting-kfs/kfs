use alloc::vec::Vec;

use crate::interrupt::apic::apic_local_vbase;

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
	InService = 0x100,
	TriggerMode = 0x180,
	InterruptRequest = 0x200,
	ErrorStatus = 0x280,
	CorrectedMachineCheckInterrupt = 0x2f0,
	InterruptCommand = 0x300,
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
	pub fn read(&self) -> Vec<usize> {
		let base = apic_local_vbase();
		match self.clone() {
			x @ (Self::InService | Self::InterruptRequest | Self::TriggerMode) => {
				mem_read(base + x as usize, 4)
			}
			x @ Self::InterruptCommand => mem_read(base + x as usize, 2),
			x => mem_read(base + x as usize, 1),
		}
	}

	pub fn write(&self, value: Vec<usize>) {
		let base = apic_local_vbase();
		mem_write(base + *self as usize, value);
	}

	pub fn iter() -> core::slice::Iter<'static, Register> {
		const REGISTERS: [Register; 25] = [
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
			Register::InService,
			Register::TriggerMode,
			Register::InterruptRequest,
			Register::ErrorStatus,
			Register::CorrectedMachineCheckInterrupt,
			Register::InterruptCommand,
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
			Self::InService => "ISR",
			Self::InitialCount => "InitialCount",
			Self::InterruptCommand => "ICR",
			Self::InterruptRequest => "IRR",
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
			Self::TriggerMode => "TMR",
			Self::Version => "Version",
		};
		write!(f, "{}", s)
	}
}

pub fn init() {}

fn mem_read(addr: usize, count: usize) -> Vec<usize> {
	(0..count)
		.into_iter()
		.map(|x| {
			let ptr = (addr + x) as *const usize;
			unsafe { *ptr }
		})
		.collect()
}

fn mem_write(addr: usize, value: Vec<usize>) {
	value.iter().enumerate().for_each(|(i, v)| {
		let ptr = (addr + i) as *mut usize;
		unsafe { *ptr = *v };
	});
}

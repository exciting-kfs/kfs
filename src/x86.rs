use core::{
	arch::asm,
	fmt::{self, Display},
	mem::size_of,
};

use crate::{
	pr_warn,
	sync::CpuLocal,
	syscall::errno::Errno,
	util::bitrange::{BitData, BitRange},
};

pub static CPU_TASK_STATE: CpuLocal<TaskState> = CpuLocal::uninit();
pub static CPU_GDT: CpuLocal<GDT> = CpuLocal::uninit();

pub const DPL_USER: usize = 0b11;
pub const DPL_KERNEL: usize = 0b00;

#[derive(Default, Debug)]
#[repr(C)]
pub struct TaskState {
	prev_task_link: usize,
	esp0: usize,
	ss0: usize,
	esp1: usize,
	ss1: usize,
	esp2: usize,
	ss2: usize,
	cr3: usize,
	eip: usize,
	eflags: usize,
	eax: usize,
	ecx: usize,
	edx: usize,
	ebx: usize,
	esp: usize,
	ebp: usize,
	esi: usize,
	edi: usize,
	es: usize,
	cs: usize,
	ss: usize,
	ds: usize,
	fs: usize,
	gs: usize,
	ldt_selector: usize,
	t: u16,
	io_map: u16,
}

impl TaskState {
	pub fn new() -> Self {
		let mut ts = Self::default();

		ts.ss0 = GDT::KERNEL_DATA;
		ts.io_map = 0x68;

		ts
	}

	pub fn change_kernel_stack(&mut self, sp: usize) {
		self.esp0 = sp;
	}
}

#[repr(packed)]
struct GDTR {
	limit: u16,
	base: *const GDT,
}

impl GDTR {
	pub fn new(target: &GDT) -> Self {
		Self {
			limit: (size_of::<GDT>() - 1) as u16,
			base: target,
		}
	}
}

#[repr(C)]
pub struct GDT {
	null: SystemDesc,
	kernel_code: SystemDesc,
	kernel_data: SystemDesc,
	user_code: SystemDesc,
	user_data: SystemDesc,
	tss: SystemDesc,
	tls1: SystemDesc,
	tls2: SystemDesc,
	tls3: SystemDesc,
}

impl GDT {
	pub const NULL: usize = 0;
	pub const KERNEL_CODE: usize = 8;
	pub const KERNEL_DATA: usize = 16;
	pub const USER_CODE: usize = 24;
	pub const USER_DATA: usize = 32;
	pub const TSS: usize = 40;
	pub const TLS1: usize = 48;
	pub const TLS2: usize = 56;
	pub const TLS3: usize = 64;

	pub fn new(tss_base: usize) -> Self {
		Self {
			null: SystemDesc::new_null(),
			kernel_code: SystemDesc::new_code(DPL_KERNEL),
			kernel_data: SystemDesc::new_data(DPL_KERNEL),
			user_code: SystemDesc::new_code(DPL_USER),
			user_data: SystemDesc::new_data(DPL_USER),
			tss: SystemDesc::new_tss(tss_base),
			tls1: SystemDesc::new_null(),
			tls2: SystemDesc::new_null(),
			tls3: SystemDesc::new_null(),
		}
	}

	pub fn pick_up(&self) {
		let gdtr = GDTR::new(self);
		unsafe { asm!("lgdt [{gdtr_pointer}]", gdtr_pointer = in(reg) &gdtr) };
	}

	pub fn load_tr(&self) {
		unsafe { asm!("ltr ax", in("ax") Self::TSS) };
	}

	pub fn load_kernel_data(&self) {
		unsafe {
			asm!(
				"mov ss, {selector}",
				"mov ds, {selector}",
				"mov es, {selector}",
				"mov fs, {selector}",
				"mov gs, {selector}",
				selector = in(reg) Self::KERNEL_DATA
			)
		};
	}

	pub fn load_kernel_code(&self) {
		unsafe {
			asm!(
				"ljmpl ${selector}, $1f",
				"1:",
				selector = const Self::KERNEL_CODE,
				options(att_syntax)
			)
		};
	}

	pub fn set_tls(&mut self, tls: &[SystemDesc; 3]) {
		self.tls1 = tls[0];
		self.tls2 = tls[1];
		self.tls3 = tls[2];
	}

	pub fn set_tls_by_idx(&mut self, idx: usize, tls: SystemDesc) -> Result<(), Errno> {
		let old_tls = match idx {
			0 => Ok(&mut self.tls1),
			1 => Ok(&mut self.tls2),
			2 => Ok(&mut self.tls3),
			_ => Err(Errno::EINVAL),
		}?;

		*old_tls = tls;

		Ok(())
	}
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct SystemDesc {
	low: BitData,
	high: BitData,
}

impl SystemDesc {
	// self.low for segment desc
	const BASE_LOW: BitRange = BitRange::new(16, 32);
	const LIMIT_LOW: BitRange = BitRange::new(0, 16);

	// self.low for call gate
	const OFFSET_LOW: BitRange = BitRange::new(0, 16);
	const SELECTOR: BitRange = BitRange::new(16, 32);

	// common self.high
	const TYPE: BitRange = BitRange::new(8, 12);
	const SYSTEM: BitRange = BitRange::new(12, 13);
	const DPL: BitRange = BitRange::new(13, 15);
	const PRESENT: BitRange = BitRange::new(15, 16);

	// self.high for segment desc
	const BASE_MID: BitRange = BitRange::new(0, 8);
	const LIMIT_HIGH: BitRange = BitRange::new(16, 20);
	const AVAILABLE: BitRange = BitRange::new(20, 21);
	const LONG: BitRange = BitRange::new(21, 22);
	const OPERATION_SIZE: BitRange = BitRange::new(22, 23);
	const GRANULARITY: BitRange = BitRange::new(23, 24);
	const BASE_HIGH: BitRange = BitRange::new(24, 32);

	// self.high for call gate
	const OFFSET_HIGH: BitRange = BitRange::new(16, 32);

	pub fn new_tss(base: usize) -> Self {
		let mut tss_desc = Self::new_null();

		tss_desc
			.set_base(base)
			.set_limit(0x67)
			.set_type(0b1001)
			.set_present(true)
			.set_dpl(DPL_KERNEL);

		tss_desc
	}

	pub fn new_data(dpl: usize) -> Self {
		let mut data = Self::new_null();

		data.set_base(0)
			.set_limit(0xfffff)
			.set_granularity(true)
			.set_operation_size(true)
			.set_system(true)
			.set_present(true)
			.set_dpl(dpl)
			.set_type(0b0011); // read write accessed

		data
	}

	pub fn new_code(dpl: usize) -> Self {
		let mut code = Self::new_null();

		code.set_base(0)
			.set_limit(0xfffff)
			.set_granularity(true)
			.set_operation_size(true)
			.set_system(true)
			.set_present(true)
			.set_dpl(dpl)
			.set_type(0b1011); // read execute non-conforming accessed

		code
	}

	pub fn new_interrupt(handler: usize, selector: usize, dpl: usize) -> Self {
		let mut interrupt = Self::new_null();

		interrupt
			.set_offset(handler)
			.set_selector(selector)
			.set_type(0b1110)
			.set_dpl(dpl)
			.set_present(true);

		interrupt
	}

	pub fn new_trap(handler: usize, selector: usize, dpl: usize) -> Self {
		let mut trap = Self::new_null();

		trap.set_offset(handler)
			.set_selector(selector)
			.set_type(0b1111)
			.set_dpl(dpl)
			.set_present(true);

		trap
	}

	pub const fn new_null() -> Self {
		Self {
			low: BitData::new(0),
			high: BitData::new(0),
		}
	}

	fn set_offset(&mut self, offset: usize) -> &mut Self {
		let l = (offset & 0x0000ffff) >> 0;
		let h = (offset & 0xffff0000) >> 16;

		self.low
			.erase_bits(&Self::OFFSET_LOW)
			.shift_add_bits(&Self::OFFSET_LOW, l);

		self.high
			.erase_bits(&Self::OFFSET_HIGH)
			.shift_add_bits(&Self::OFFSET_HIGH, h);

		self
	}

	fn set_selector(&mut self, sel: usize) -> &mut Self {
		self.low
			.erase_bits(&Self::SELECTOR)
			.shift_add_bits(&Self::SELECTOR, sel);

		self
	}

	fn set_base(&mut self, base: usize) -> &mut Self {
		let l = (base & 0x0000ffff) >> 0;
		let m = (base & 0x00ff0000) >> 16;
		let h = (base & 0xff000000) >> 24;

		self.low
			.erase_bits(&Self::BASE_LOW)
			.shift_add_bits(&Self::BASE_LOW, l);

		self.high
			.erase_bits(&Self::BASE_MID)
			.erase_bits(&Self::BASE_HIGH)
			.shift_add_bits(&Self::BASE_MID, m)
			.shift_add_bits(&Self::BASE_HIGH, h);

		self
	}

	fn set_limit(&mut self, limit: usize) -> &mut Self {
		let l = (limit & 0b0000_0000_0000_0000_1111_1111_1111_1111) >> 0;
		let h = (limit & 0b0000_0000_0000_1111_0000_0000_0000_0000) >> 16;

		self.low
			.erase_bits(&Self::LIMIT_LOW)
			.shift_add_bits(&Self::LIMIT_LOW, l);

		self.high
			.erase_bits(&Self::LIMIT_HIGH)
			.shift_add_bits(&Self::LIMIT_HIGH, h);

		self
	}

	fn set_type(&mut self, desc_type: usize) -> &mut Self {
		self.high
			.erase_bits(&Self::TYPE)
			.shift_add_bits(&Self::TYPE, desc_type);

		self
	}

	fn set_system(&mut self, system: bool) -> &mut Self {
		self.high
			.erase_bits(&Self::SYSTEM)
			.shift_add_bits(&Self::SYSTEM, system.into());

		self
	}

	fn set_dpl(&mut self, dpl: usize) -> &mut Self {
		self.high
			.erase_bits(&Self::DPL)
			.shift_add_bits(&Self::DPL, dpl);

		self
	}

	fn set_present(&mut self, present: bool) -> &mut Self {
		self.high
			.erase_bits(&Self::PRESENT)
			.shift_add_bits(&Self::PRESENT, present.into());

		self
	}

	fn set_available(&mut self, available: bool) -> &mut Self {
		self.high
			.erase_bits(&Self::AVAILABLE)
			.shift_add_bits(&Self::AVAILABLE, available.into());

		self
	}

	fn set_long_mode(&mut self, long_mode: bool) -> &mut Self {
		self.high
			.erase_bits(&Self::LONG)
			.shift_add_bits(&Self::LONG, long_mode.into());

		self
	}

	fn set_operation_size(&mut self, operation_size: bool) -> &mut Self {
		self.high
			.erase_bits(&Self::OPERATION_SIZE)
			.shift_add_bits(&Self::OPERATION_SIZE, operation_size.into());

		self
	}

	fn set_granularity(&mut self, graularity: bool) -> &mut Self {
		self.high
			.erase_bits(&Self::GRANULARITY)
			.shift_add_bits(&Self::GRANULARITY, graularity.into());

		self
	}
}

impl Display for SystemDesc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"0x{:08x}{:08x}",
			self.high.get_raw_bits(),
			self.low.get_raw_bits()
		)
	}
}

#[repr(transparent)]
pub struct UserDescFlag(BitData);

impl UserDescFlag {
	const SEG_32BIT: BitRange = BitRange::new(0, 1);
	const CONTENTS: BitRange = BitRange::new(1, 3);
	const READ_EXEC_ONLY: BitRange = BitRange::new(3, 4);
	const LIMIT_IN_PAGES: BitRange = BitRange::new(4, 5);
	const SEG_NOT_PRESENT: BitRange = BitRange::new(5, 6);
	const USEABLE: BitRange = BitRange::new(6, 7);

	pub fn is_empty(&self) -> bool {
		let raw = self.0.get_raw_bits() & ((1 << 7) - 1);

		// SEG_NOT_PRESENT | READ_EXEC_ONLY
		raw == ((1 << 5) | (1 << 3))
	}
}

impl fmt::Debug for UserDescFlag {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{{ SEG_32BIT: {}, CONTENTS: {}, READ_EXEC_ONLY: {}, LIMIT_IN_PAGES: {}, SEG_NOT_PRESENT: {}, USABLE: {} }}",
			self.0.shift_get_bits(&Self::SEG_32BIT),
			self.0.shift_get_bits(&Self::CONTENTS),
			self.0.shift_get_bits(&Self::READ_EXEC_ONLY),
			self.0.shift_get_bits(&Self::LIMIT_IN_PAGES),
			self.0.shift_get_bits(&Self::SEG_NOT_PRESENT),
			self.0.shift_get_bits(&Self::USEABLE)
		)
	}
}

#[repr(C)]
#[derive(Debug)]
pub struct UserDesc {
	pub entry_number: i32,
	base_addr: usize,
	limit: usize,
	flags: UserDescFlag,
}

impl UserDesc {
	pub fn is_empty(&self) -> bool {
		self.base_addr == 0 && self.limit == 0 && self.flags.is_empty()
	}

	pub fn parse_into_system_desc(&self) -> Result<SystemDesc, Errno> {
		if self.is_empty() {
			return Ok(SystemDesc::new_null());
		}

		let contents = self.flags.0.shift_get_bits(&UserDescFlag::CONTENTS);
		if contents != 0 {
			pr_warn!("UserDesc::parse: unknown contents: {}", contents);
		}

		let mut desc = SystemDesc::new_null();

		let present = self.flags.0.get_bits(&UserDescFlag::SEG_NOT_PRESENT) == 0;
		let granularity = self.flags.0.get_bits(&UserDescFlag::LIMIT_IN_PAGES) != 0;
		let operation_size = self.flags.0.get_bits(&UserDescFlag::SEG_32BIT) != 0;
		let writable = self.flags.0.get_bits(&UserDescFlag::READ_EXEC_ONLY) != 0;

		desc.set_base(self.base_addr)
			.set_limit(self.limit)
			.set_granularity(granularity)
			.set_operation_size(operation_size)
			.set_system(true)
			.set_present(present)
			.set_dpl(3)
			.set_type(0b0001 | ((writable as usize) << 1)); // read accessed | write

		Ok(desc)
	}
}

pub fn get_eflags() -> usize {
	let eflags;

	unsafe {
		asm!(
			"pushfd",
			"pop {eflags}",
			eflags = out(reg) eflags)
	};

	eflags
}

pub unsafe fn init() {
	CPU_TASK_STATE.init(TaskState::new());

	CPU_GDT.init(GDT::new(
		(&*CPU_TASK_STATE.get_mut()) as *const TaskState as usize,
	));

	let gdt = CPU_GDT.get_mut();

	gdt.pick_up();
	gdt.load_kernel_code();
	gdt.load_kernel_data();
	gdt.load_tr();
}

use crate::util::bit_range::BitRange;

use super::privilege_level::PrivilegeLevel;

// FIXME hmm?
const CODE_SELECTOR_KERNEL: usize = 0x8;
const CODE_SELECTOR_USER: usize = 0x18;

#[repr(u8)]
enum EntryKind {
	Task = 0b0101,
	Interrupt = 0b1110,
	Trap = 0b1111,
}

#[repr(C, packed(8))]
#[derive(Clone, Copy, Debug)]
pub struct IDTE {
	low: usize,
	high: usize,
}

impl IDTE {
	const L_OFFSET: BitRange = BitRange::new(0, 16);
	const L_SEGMENT_SELECTOR: BitRange = BitRange::new(16, 32);
	const H_KIND: BitRange = BitRange::new(8, 12);
	const H_DPL: BitRange = BitRange::new(13, 15);
	const H_PRESENT: BitRange = BitRange::new(15, 16);
	const H_OFFSET: BitRange = BitRange::new(16, 32);

	pub const fn null() -> Self {
		Self { low: 0, high: 0 }
	}

	fn segment_selector(&mut self, sel: usize) {
		self.low &= !Self::L_SEGMENT_SELECTOR.mask();
		self.low |= Self::L_SEGMENT_SELECTOR.fit(sel);
	}

	fn offset(&mut self, handler: usize) {
		self.low &= !Self::L_OFFSET.mask();
		self.low |= Self::L_OFFSET.fit(handler);

		self.high &= !Self::H_OFFSET.mask();
		self.high |= Self::H_OFFSET.fit(handler >> 16);
	}

	fn kind(&mut self, kind: EntryKind) {
		self.high &= !Self::H_KIND.mask();
		self.high |= Self::H_KIND.fit(kind as usize);
	}

	fn privilege_level(&mut self, level: PrivilegeLevel) {
		self.high &= !Self::H_DPL.mask();
		self.high |= Self::H_DPL.fit(level as usize);
	}

	fn present(&mut self, p: usize) {
		self.high &= !Self::H_PRESENT.mask();
		self.high |= Self::H_PRESENT.fit(p);
	}

	fn set(&mut self, sel: usize, handler: usize, kind: EntryKind, level: PrivilegeLevel) {
		self.segment_selector(sel);
		self.offset(handler);
		self.kind(kind);
		self.privilege_level(level);
		self.present(1);
	}

	pub fn interrupt_kernel(handler: usize) -> Self {
		let mut entry = IDTE::null();

		entry.set(
			CODE_SELECTOR_KERNEL,
			handler,
			EntryKind::Interrupt,
			PrivilegeLevel::Kernel,
		);

		entry
	}

	pub fn interrupt_user(handler: usize) -> Self {
		let mut entry = IDTE::null();

		entry.set(
			CODE_SELECTOR_USER,
			handler,
			EntryKind::Interrupt,
			PrivilegeLevel::User,
		);

		entry
	}

	pub fn trap_kernel(handler: usize) -> Self {
		let mut entry = IDTE::null();

		entry.set(
			CODE_SELECTOR_KERNEL,
			handler,
			EntryKind::Trap,
			PrivilegeLevel::Kernel,
		);

		entry
	}

	pub fn trap_user(handler: usize) -> Self {
		let mut entry = IDTE::null();

		entry.set(
			CODE_SELECTOR_USER,
			handler,
			EntryKind::Trap,
			PrivilegeLevel::User,
		);

		entry
	}

	pub fn task_kernel(seg_sel: usize) -> Self {
		let mut entry = IDTE::null();

		entry.segment_selector(seg_sel);
		entry.kind(EntryKind::Task);
		entry.privilege_level(PrivilegeLevel::Kernel);
		entry.present(1);

		entry
	}

	pub fn task_user(seg_sel: usize) -> Self {
		let mut entry = IDTE::null();

		entry.segment_selector(seg_sel);
		entry.kind(EntryKind::Task);
		entry.privilege_level(PrivilegeLevel::User);
		entry.present(1);

		entry
	}
}

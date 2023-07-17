use bitflags::bitflags;

bitflags! {
	#[repr(transparent)]
	#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
	pub struct PageFlag: u32 {
		const Present = 1;
		const Write = 2;
		const User = 4;
		const PWT = 8;
		const PCD = 16;
		const Accessed = 32;
		const Dirty = 64;
		const PAT = 128;
		const Global = 256;
	}
}

impl PageFlag {
	pub const USER_RDWR: Self = Self::Present.union(Self::User).union(Self::Write);
	pub const USER_RDONLY: Self = Self::Present.union(Self::User);
	pub const KERNEL_RDWR: Self = Self::Present.union(Self::Global).union(Self::Write);
	pub const KERNEL_RDONLY: Self = Self::Present.union(Self::Global);
}

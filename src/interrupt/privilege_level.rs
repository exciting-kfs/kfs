#[repr(u8)]
pub enum PrivilegeLevel {
	Kernel = 0b00,
	User = 0b11,
}

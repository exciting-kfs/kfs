mod cache;
pub mod page;
pub mod phys;
pub mod virt;

pub enum Zone {
	Normal,
	High,
}

pub enum GFP {
	Normal,
	Atomic,
}

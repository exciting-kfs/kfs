mod cache;
pub mod page;
pub mod phys;
pub mod virt;

#[derive(Clone)]
pub enum Zone {
	Normal,
	High,
}

#[derive(Clone)]
pub enum GFP {
	Normal,
	Atomic,
}

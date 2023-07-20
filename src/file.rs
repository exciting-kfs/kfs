pub mod read;
pub mod write;

use alloc::sync::Arc;

pub struct File {
	pub open_flag: usize,
	pub ops: Arc<dyn FileOps>,
}

pub trait FileOps {
	fn read(&self, buf: &mut [u8]) -> usize;
	fn write(&self, buf: &[u8]) -> usize;
}

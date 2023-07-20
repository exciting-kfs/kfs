pub mod block;
pub mod character;
pub mod pmio;

pub use block::{Read as BlkRead, Write as BlkWrite};
pub use character::{Read as ChRead, Write as ChWrite};

#[derive(Debug)]
pub struct NoSpace;

//! Constants for buddy allocator

pub use super::super::constant::*;

pub const MAX_RANK: usize = 10;
/// Maximum allocation size per request.
pub const BLOCK_SIZE: usize = PAGE_SIZE * (1 << MAX_RANK);

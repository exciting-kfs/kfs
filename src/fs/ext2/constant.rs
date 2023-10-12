use crate::mm::constant::{KB, MB};

pub(super) const MAX_CACHED_BLOCK_BYTE: usize = KB * MB;
pub(super) const SYNC_INTERVAL: u32 = 10;

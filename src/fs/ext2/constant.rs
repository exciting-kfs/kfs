use crate::mm::constant::{KB, MB};

const MAX_CACHED_BLOCK_BYTE: usize = MB * KB;

pub(super) const MAX_CACHED_BLOCK: usize = MAX_CACHED_BLOCK_BYTE / 1000;
pub(super) const SYNC_INTERVAL: u32 = 10;

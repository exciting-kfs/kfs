mod cache_manager;
mod meta_cache;
mod size_cache;
mod traits;
mod util;

pub const REGISTER_TRY: usize = 3; // TODO config?

pub use cache_manager::CM;
pub use size_cache::{SizeCache, SizeCacheTrait};
pub use util::{alloc_block_from_page_alloc, dealloc_block_to_page_alloc};

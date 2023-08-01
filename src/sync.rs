mod lock;
pub use lock::spinlock;
pub use lock::TryLockFail;

pub mod cpu_local;
pub mod locked;

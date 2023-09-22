mod spinlock;

pub use spinlock::*;

#[derive(Debug)]
pub struct TryLockFail;

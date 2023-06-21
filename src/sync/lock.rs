pub mod spinlock;

pub struct TryLockFail;

pub enum LockType {
	Default,
	Irq,
	IrqSave,
}

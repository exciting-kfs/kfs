#[macro_export]
macro_rules! atomic_operation {
	($($tt:tt)*) => {
		{
			let __irq_save = crate::sync::spinlock::irq_save();
			$($tt)*
		}
	};
}

mod test {
	use kfs_macro::ktest;

	use crate::sync::locked::Locked;

	struct A(usize);
	struct B(isize);
	struct C(usize);

	static LOCK_A1: Locked<A> = Locked::new(A(1));
	static LOCK_B1: Locked<B> = Locked::new(B(1));
	static LOCK_C1: Locked<C> = Locked::new(C(2));

	#[ktest(atomic_op)]
	fn test() {
		fn func() -> usize {
			let _c_lock = LOCK_C1.lock();
			atomic_operation! {
				let a = LOCK_A1.lock().0;
				let b = LOCK_B1.lock().0 as usize;
				assert_eq!(a + b, 2);
				a + b
			}
		}

		let _ret = func();
	}
}

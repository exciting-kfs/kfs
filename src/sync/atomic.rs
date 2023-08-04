#[macro_export]
macro_rules! atomic_operation {
	($($tt:tt)*) => {
		{
			let __irq_save = crate::sync::spinlock::irq_save();
			$($tt)*
		};
		let __final_line_guard = 0;
	};
}

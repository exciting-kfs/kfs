#[macro_export]
macro_rules! trace_feature {
	($($feat:literal $(|)?)*, $($arg:tt)*) => {
		#[cfg(all(log_level = "debug", any($(trace_feature = $feat),*)))]
		{
			$(
				if cfg!(trace_feature = $feat) {
				    $crate::printk!("{}: ", $feat);
				}
			)*

			let path = core::module_path!();
			if let Some(pos) = path.find("::") {
				$crate::printk!("<{}> ", &path[(pos + 2)..]);
			}

			$crate::printkln!($($arg)*);
		}
	};
}

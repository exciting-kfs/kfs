macro_rules! include_user_bin {
	($name:literal) => {
		include_bytes!(concat!("../userspace/build/", $name))
	};
}

macro_rules! define_user_bin {
	[$(($varname:ident, $filename:literal)),*] => {
		$(
			pub static $varname: &'static [u8] = include_user_bin!($filename);
		)*
		pub fn get_user_bin(name: &str) -> Option<&'static [u8]> {
			match name {
				$($filename => Some($varname),)*
				_ => None,
			}
		}
	};
}

// it will define pub fn get_user_bin
define_user_bin![
	(INIT, "init.bin"),
	(SHELL, "shell.bin"),
	(TEST_PIPE, "test_pipe.bin"),
	(TEST_SIG, "test_sig.bin"),
	(TEST_SETXID, "test_setXid.bin"),
	(TEST_SIGSTOPCONT, "test_sig_stop_cont.bin")
];

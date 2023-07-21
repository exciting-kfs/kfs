macro_rules! include_user_bin {
	($name:literal) => {
		include_bytes!(concat!("../userspace/build/", $name))
	};
}

pub static INIT_CODE: &'static [u8] = include_user_bin!("init.bin");
pub static SHELL: &'static [u8] = include_user_bin!("shell.bin");

use crate::{
	elf::{Elf, ElfError},
	syscall::errno::Errno,
};

macro_rules! include_user_bin {
	($name:literal) => {{
		#[repr(C, align(16))]
		struct __Align16<T: ?Sized>(T);

		static __ALIGNED: &'static __Align16<[u8]> =
			&__Align16(*include_bytes!(concat!("../userspace/build/", $name)));

		&__ALIGNED.0
	}};
}

macro_rules! define_user_bin {
	[$(($varname:ident, $filename:literal)),* $(,)?] => {
		$(
			pub static $varname: &'static [u8] = include_user_bin!($filename);
		)*

		fn get_user_bin(name: &str) -> Option<&'static [u8]> {
			match name {
				$($filename => Some($varname),)*
				_ => None,
			}
		}
	};
}

define_user_bin![
	(INIT, "init"),
	(SHELL, "shell"),
	(TEST_PIPE, "test_pipe"),
	(TEST_SIG, "test_sig"),
	(TEST_SETXID, "test_setXid"),
	(TEST_SIGSTOPCONT, "test_sig_stop_cont"),
	(TEST_FILE, "test_file"),
	(TEST_SOCKET, "test_socket"),
	(GETTY, "getty"),
	(TEST, "test"),
	(TEST_ARGV, "test_argv"),
];

pub fn get_user_elf(name: &str) -> Result<Elf<'_>, Errno> {
	let user_bin = get_user_bin(name).ok_or(Errno::ENOENT)?;

	Elf::new(user_bin).map_err(|x| <ElfError as Into<Errno>>::into(x))
}

use crate::{driver::ps2, fs, test::exit_qemu_with};

use super::errno::Errno;

// LINUX_REBOOT_CMD_POWER_OFF
// (RB_POWER_OFF, 0x4321fedc; since Linux 2.1.30)

// LINUX_REBOOT_CMD_RESTART
// (RB_AUTOBOOT, 0x1234567)

enum Cmd {
	PowerOff = 0x4321fedc,
	Restart = 0x1234567,
}

impl Cmd {
	fn from_usize(v: usize) -> Result<Cmd, Errno> {
		match v {
			0x4321fedc => Ok(Cmd::PowerOff),
			0x1234567 => Ok(Cmd::Restart),
			_ => Err(Errno::EINVAL),
		}
	}
}

pub fn sys_reboot(cmd: usize) -> Result<usize, Errno> {
	let cmd = Cmd::from_usize(cmd)?;

	fs::clean_up()?;

	match cmd {
		Cmd::PowerOff => power_off(),
		Cmd::Restart => restart(),
	}

	Ok(0)
}

fn power_off() {
	// TODO power off on actual computer.
	exit_qemu_with(0);
}

fn restart() {
	ps2::control::reset_cpu();
}

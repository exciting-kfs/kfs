pub const TCGETS: u32 = 21505;
pub const TCSETS: u32 = 21506;
pub const TCSETSW: u32 = 21507;
pub const TCSETSF: u32 = 21508;
pub const TCGETA: u32 = 21509;
pub const TCSETA: u32 = 21510;
pub const TCSETAW: u32 = 21511;
pub const TCSETAF: u32 = 21512;
pub const TCSBRK: u32 = 21513;
pub const TCXONC: u32 = 21514;
pub const TCFLSH: u32 = 21515;
pub const TIOCEXCL: u32 = 21516;
pub const TIOCNXCL: u32 = 21517;
pub const TIOCSCTTY: u32 = 21518;
pub const TIOCGPGRP: u32 = 21519;
pub const TIOCSPGRP: u32 = 21520;
pub const TIOCOUTQ: u32 = 21521;
pub const TIOCSTI: u32 = 21522;
pub const TIOCGWINSZ: u32 = 21523;
pub const TIOCSWINSZ: u32 = 21524;
pub const TIOCMGET: u32 = 21525;
pub const TIOCMBIS: u32 = 21526;
pub const TIOCMBIC: u32 = 21527;
pub const TIOCMSET: u32 = 21528;
pub const TIOCGSOFTCAR: u32 = 21529;
pub const TIOCSSOFTCAR: u32 = 21530;
pub const FIONREAD: u32 = 21531;
pub const TIOCINQ: u32 = 21531;
pub const TIOCLINUX: u32 = 21532;
pub const TIOCCONS: u32 = 21533;
pub const TIOCGSERIAL: u32 = 21534;
pub const TIOCSSERIAL: u32 = 21535;
pub const TIOCPKT: u32 = 21536;
pub const FIONBIO: u32 = 21537;
pub const TIOCNOTTY: u32 = 21538;
pub const TIOCSETD: u32 = 21539;
pub const TIOCGETD: u32 = 21540;
pub const TCSBRKP: u32 = 21541;
pub const TIOCSBRK: u32 = 21543;
pub const TIOCCBRK: u32 = 21544;
pub const TIOCGSID: u32 = 21545;
pub const TIOCGRS485: u32 = 21550;
pub const TIOCSRS485: u32 = 21551;
pub const TIOCGPTN: u32 = 2147767344;
pub const TIOCSPTLCK: u32 = 1074025521;
pub const TIOCGDEV: u32 = 2147767346;
pub const TCGETX: u32 = 21554;
pub const TCSETX: u32 = 21555;
pub const TCSETXF: u32 = 21556;
pub const TCSETXW: u32 = 21557;
pub const TIOCSIG: u32 = 1074025526;
pub const TIOCVHANGUP: u32 = 21559;
pub const TIOCGPKT: u32 = 2147767352;
pub const TIOCGPTLCK: u32 = 2147767353;
pub const TIOCGEXCL: u32 = 2147767360;
pub const TIOCGPTPEER: u32 = 21569;
pub const TIOCGISO7816: u32 = 2150126658;
pub const TIOCSISO7816: u32 = 3223868483;
pub const FIONCLEX: u32 = 21584;
pub const FIOCLEX: u32 = 21585;
pub const FIOASYNC: u32 = 21586;
pub const TIOCSERCONFIG: u32 = 21587;
pub const TIOCSERGWILD: u32 = 21588;
pub const TIOCSERSWILD: u32 = 21589;
pub const TIOCGLCKTRMIOS: u32 = 21590;
pub const TIOCSLCKTRMIOS: u32 = 21591;
pub const TIOCSERGSTRUCT: u32 = 21592;
pub const TIOCSERGETLSR: u32 = 21593;
pub const TIOCSERGETMULTI: u32 = 21594;
pub const TIOCSERSETMULTI: u32 = 21595;
pub const TIOCMIWAIT: u32 = 21596;
pub const TIOCGICOUNT: u32 = 21597;
pub const FIOQSIZE: u32 = 21600;
pub const TIOCM_LE: u32 = 1;
pub const TIOCM_DTR: u32 = 2;
pub const TIOCM_RTS: u32 = 4;
pub const TIOCM_ST: u32 = 8;
pub const TIOCM_SR: u32 = 16;
pub const TIOCM_CTS: u32 = 32;
pub const TIOCM_CAR: u32 = 64;
pub const TIOCM_RNG: u32 = 128;
pub const TIOCM_DSR: u32 = 256;
pub const TIOCM_CD: u32 = 64;
pub const TIOCM_RI: u32 = 128;
pub const TIOCM_OUT1: u32 = 8192;
pub const TIOCM_OUT2: u32 = 16384;
pub const TIOCM_LOOP: u32 = 32768;
pub const FIOSETOWN: u32 = 35073;
pub const SIOCSPGRP: u32 = 35074;
pub const FIOGETOWN: u32 = 35075;
pub const SIOCGPGRP: u32 = 35076;
pub const SIOCATMARK: u32 = 35077;
pub const SIOCGSTAMP: u32 = 35078;
pub const SIOCGSTAMPNS: u32 = 35079;

pub const VINTR: usize = 0;
pub const VQUIT: usize = 1;
pub const VERASE: usize = 2;
pub const VKILL: usize = 3;
pub const VEOF: usize = 4;
pub const VTIME: usize = 5;
pub const VMIN: usize = 6;
pub const VSWTC: usize = 7;
pub const VSTART: usize = 8;
pub const VSTOP: usize = 9;
pub const VSUSP: usize = 10;
pub const VEOL: usize = 11;
pub const VREPRINT: usize = 12;
pub const VDISCARD: usize = 13;
pub const VWERASE: usize = 14;
pub const VLNEXT: usize = 15;
pub const VEOL2: usize = 16;

#[repr(C)]
pub struct WinSize {
	pub row: u16,
	pub col: u16,
}

use bitflags::bitflags;

use super::ascii::constants::{DEL, EOF, ETX, FS};

bitflags! {
	#[repr(transparent)]
	#[derive(Clone, Copy, Debug)]
	pub struct LocalFlag: u32 {
		const ISIG = 0o000001;
		const ICANON = 0o000002;
		const XCASE = 0o000004;
		const ECHO = 0o000010;
		const ECHOE = 0o000020;
		const ECHOK = 0o000040;
		const ECHONL = 0o000100;
		const NOFLSH = 0o000200;
		const TOSTOP = 0o000400;
		const ECHOCTL = 0o001000;
		const ECHOPRT = 0o002000;
		const ECHOKE = 0o004000;
		const FLUSHO = 0o010000;
		const PENDIN = 0o040000;
		const IEXTEN = 0o100000;
		const EXTPROC = 0o200000;
	}
}

bitflags! {
	#[repr(transparent)]
	#[derive(Clone, Copy, Debug)]
	pub struct InputFlag: u32 {
		const IGNBRK = 0o000001;
		const BRKINT = 0o000002;
		const IGNPAR = 0o000004;
		const PARMRK = 0o000010;
		const INPCK = 0o000020;
		const ISTRIP = 0o000040;
		const INLCR = 0o000100;
		const IGNCR = 0o000200;
		const ICRNL = 0o000400;
		const IUCLC = 0o001000;
		const IXON = 0o002000;
		const IXANY = 0o004000;
		const IXOFF = 0o010000;
		const IMAXBEL = 0o020000;
		const IUTF8 = 0o040000;
	}
}

bitflags! {
	#[repr(transparent)]
	#[derive(Clone, Copy, Debug)]
	pub struct OutputFlag: u32 {
		const OPOST = 0o000001;
		const OLCUC = 0o000002;
		const ONLCR = 0o000004;
		const OCRNL = 0o000010;
		const ONOCR = 0o000020;
		const ONLRET = 0o000040;
		const OFILL = 0o000100;
		const OFDEL = 0o000200;
		const NLDLY = 0o000400;
		const NL0 = 0o000000;
		const NL1 = 0o000400;
		const CRDLY = 0o003000;
		const CR0 = 0o000000;
		const CR1 = 0o001000;
		const CR2 = 0o002000;
		const CR3 = 0o003000;
		const TABDLY = 0o014000;
		const TAB0 = 0o000000;
		const TAB1 = 0o004000;
		const TAB2 = 0o010000;
		const TAB3 = 0o014000;
		const XTABS = 0o014000;
		const BSDLY = 0o020000;
		const BS0 = 0o000000;
		const BS1 = 0o020000;
		const VTDLY = 0o040000;
		const VT0 = 0o000000;
		const VT1 = 0o040000;
		const FFDLY = 0o100000;
		const FF0 = 0o000000;
		const FF1 = 0o100000;
	}
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Termios {
	pub iflag: InputFlag,
	pub oflag: OutputFlag,
	pub cflag: u32,
	pub lflag: LocalFlag,
	pub line_disc: u8,
	pub control_char: [u8; 19],
	pub ispeed: u32,
	pub ospeed: u32,
}

impl Termios {
	pub const RAW: Self = Self {
		iflag: InputFlag::empty(),
		oflag: OutputFlag::empty(),
		cflag: 0,
		lflag: LocalFlag::ECHO.union(LocalFlag::ECHOCTL),
		line_disc: 0,
		control_char: [
			ETX, FS, DEL, 0x15, EOF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		],
		ispeed: 0,
		ospeed: 0,
	};

	pub const SANE: Self = Self {
		iflag: InputFlag::ICRNL,
		oflag: OutputFlag::OPOST.union(OutputFlag::ONLCR),
		cflag: 0,
		lflag: LocalFlag::ECHO
			.union(LocalFlag::ECHOCTL)
			.union(LocalFlag::ECHOE)
			.union(LocalFlag::ECHOK)
			.union(LocalFlag::ICANON)
			.union(LocalFlag::ISIG),
		line_disc: 0,
		control_char: [
			ETX, FS, DEL, 0x15, EOF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		],
		ispeed: 0,
		ospeed: 0,
	};
}

use core::{
	fmt::{Display, LowerHex},
	mem::MaybeUninit,
	ops::Deref,
	slice::Iter,
};

use alloc::boxed::Box;

use crate::{mm::constant::SECTOR_SIZE, pr_debug, sync::locked::Locked};

use super::{dev_num::DevNum, get_ide_controller, lba::LBA28};

#[repr(C)]
#[derive(Debug)]
pub struct PartitionEntry {
	attribute: u8,
	begin_h: u8,
	begin_s: u8,
	begin_c: u8,
	pub partition_type: PartitionType,
	last_h: u8,
	last_s: u8,
	last_c: u8,
	begin_lba: u32,
	sector_count: u32,
}

impl PartitionEntry {
	pub fn begin(&self) -> LBA28 {
		debug_assert!(self.partition_type != PartitionType::Empty);
		LBA28::new(self.begin_lba as usize)
	}

	pub fn end(&self) -> LBA28 {
		debug_assert!(self.partition_type != PartitionType::Empty);
		let (c, h, s) = (self.last_c, self.last_h, self.last_s);
		LBA28::from_chs(c, h, s) + 1
	}
}

#[derive(Debug)]
pub struct PartitionTable([PartitionEntry; 4]);

impl PartitionTable {
	pub fn iter(&self) -> Iter<'_, PartitionEntry> {
		self.0.iter()
	}
}

impl Deref for PartitionTable {
	type Target = [PartitionEntry; 4];
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Display for PartitionTable {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		for (i, e) in self.0.iter().enumerate() {
			if e.partition_type == PartitionType::Empty {
				continue;
			}
			write!(f, "Entry[{i}]:\n")?;
			write!(f, "\tattr: {:x},", e.attribute)?;
			write!(f, "\ttype: {:x}\n", e.partition_type)?;
			write!(
				f,
				"\tbegin CHS: ({:x}, {:x}, {:x})\n",
				e.begin_c, e.begin_h, e.begin_s
			)?;
			write!(
				f,
				"\tlast  CHS: ({:x}, {:x}, {:x})\n",
				e.last_c, e.last_h, e.last_s
			)?;
			write!(f, "\tbegin LBA: {:x}\n", e.begin_lba)?;
			write!(
				f,
				"\tlast  LBA: {:x}\n",
				LBA28::from_chs(e.last_c, e.last_h, e.last_s)
			)?;
			write!(f, "\tsector count: {:x}\n", e.sector_count)?;
		}
		Ok(())
	}
}

pub static PART_TABLE: [Locked<Option<Box<PartitionTable>>>; 4] = [
	Locked::new(None),
	Locked::new(None),
	Locked::new(None),
	Locked::new(None),
];

const BOOT_SECTOR_MAGIC: u16 = 0xaa55;
const BOOT_SECTOR_OFFSET: usize = 0x1fe / 2;
const PARTITION_TABLE_OFFSET: usize = 0x1be / 2;

fn read_partition_table(dev: DevNum) -> Option<Box<PartitionTable>> {
	let ide = get_ide_controller(dev);

	let mut sector = Box::new_uninit_slice(1);
	ide.read_sectors(LBA28::new(0), &mut sector);

	let sector = unsafe { sector.assume_init() };

	if sector[0][BOOT_SECTOR_OFFSET] != BOOT_SECTOR_MAGIC {
		return None;
	}

	let mut part_table: Box<MaybeUninit<PartitionTable>> = Box::new_uninit();
	unsafe {
		part_table
			.as_mut_ptr()
			.cast::<u16>()
			.copy_from_nonoverlapping(&sector[0][PARTITION_TABLE_OFFSET], 32)
	};

	Some(unsafe { part_table.assume_init() })
}

pub fn init(devices: [Option<DevNum>; 4]) {
	for (dev, entry) in devices.into_iter().zip(PART_TABLE.iter()) {
		if let Some(dev) = dev {
			*entry.lock() = read_partition_table(dev);
		}
	}
}

pub fn byte_to_sector_count(byte: usize) -> usize {
	(byte - 1) / SECTOR_SIZE + 1
}

fn print_partition_table() {
	for (i, tab) in PART_TABLE.iter().enumerate() {
		if let Some(t) = &*tab.lock() {
			pr_debug!("{}:\n{}", i, t);
		}
	}
}

/// From fdisk & [Partition Type](https://en.wikipedia.org/wiki/Partition_type)
#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum PartitionType {
	Empty = 0x00,              // Empty
	FAT12 = 0x01,              // FAT12
	XENIXroot = 0x02,          // XENIX root
	XENIXusr = 0x03,           // XENIX usr
	FAT16_32M = 0x04,          // FAT16 <32M
	Extended = 0x05,           // Extended
	FAT16 = 0x06,              // FAT16
	HPFS_NTFS_exFAT = 0x07,    // HPFS/NTFS/exFAT
	AIX = 0x08,                // AIX
	AIXbootable = 0x09,        // AIX bootable
	OS_2BootManag = 0x0a,      // OS/2 Boot Manag
	W95FAT32 = 0x0b,           // W95 FAT32
	W95FAT32_LBA = 0x0c,       // W95 FAT32 (LBA)
	W95FAT16_LBA = 0x0e,       // W95 FAT16 (LBA)
	W95Ext_d_LBA = 0x0f,       // W95 Ext'd (LBA)
	OPUS = 0x10,               // OPUS
	HiddenFAT12 = 0x11,        // Hidden FAT12
	Compaqdiagnost = 0x12,     // Compaq diagnost
	HiddenFAT16 = 0x14,        // Hidden FAT16 <3
	HiddenFAT16B = 0x16,       // Hidden FAT16
	HiddenHPFS_NTF = 0x17,     // Hidden HPFS/NTF
	ASTSmartSleep = 0x18,      // AST SmartSleep
	HiddenW95FAT32 = 0x1b,     // Hidden W95 FAT3
	HiddenW95FAT32_LBA = 0x1c, // Hidden W95 FAT3
	HiddenW95FAT16 = 0x1e,     // Hidden W95 FAT1
	NECDOS = 0x24,             // NEC DOS
	HiddenNTFSWin = 0x27,      // Hidden NTFS Win
	Plan9 = 0x39,              // Plan 9
	PartitionMagic = 0x3c,     // PartitionMagic
	Venix80286 = 0x40,         // Venix 80286
	PPCPRePBoot = 0x41,        // PPC PReP Boot
	SFS = 0x42,                // SFS
	QNX4_x = 0x4d,             // QNX4.x
	QNX4_x2ndpart = 0x4e,      // QNX4.x 2nd part
	QNX4_x3rdpart = 0x4f,      // QNX4.x 3rd part
	OnTrackDM = 0x50,          // OnTrack DM
	OnTrackDM6Aux0 = 0x51,     // OnTrack DM6 Aux
	CP_M = 0x52,               // CP/M
	OnTrackDM6Aux1 = 0x53,     // OnTrack DM6 Aux
	OnTrackDM6 = 0x54,         // OnTrackDM6
	EZ_Drive = 0x55,           // EZ-Drive
	GoldenBow = 0x56,          // Golden Bow
	PriamEdisk = 0x5c,         // Priam Edisk
	SpeedStor = 0x61,          // SpeedStor
	GNUHURDorSys = 0x63,       // GNU HURD or Sys
	NovellNetware0 = 0x64,     // Novell Netware
	NovellNetware1 = 0x65,     // Novell Netware
	DiskSecureMult = 0x70,     // DiskSecure Mult
	PC_IX = 0x75,              // PC/IX
	OldMinix = 0x80,           // Old Minix
	Minix_oldLin = 0x81,       // Minix / old Lin
	LinuxSwapSo = 0x82,        // Linux swap / So
	Linux = 0x83,              // Linux
	OS_2hiddenor = 0x84,       // OS/2 hidden or
	Linuxextended0 = 0x85,     // Linux extended
	NTFSvolumeset0 = 0x86,     // NTFS volume set
	NTFSvolumeset1 = 0x87,     // NTFS volume set
	Linuxplaintext = 0x88,     // Linux plaintext
	LinuxLVM = 0x8e,           // Linux LVM
	Amoeba = 0x93,             // Amoeba
	AmoebaBBT = 0x94,          // Amoeba BBT
	BSD_OS = 0x9f,             // BSD/OS
	IBMThinkpadhi = 0xa0,      // IBM Thinkpad hi
	FreeBSD = 0xa5,            // FreeBSD
	OpenBSD = 0xa6,            // OpenBSD
	NeXTSTEP = 0xa7,           // NeXTSTEP
	DarwinUFS = 0xa8,          // Darwin UFS
	NetBSD = 0xa9,             // NetBSD
	Darwinboot = 0xab,         // Darwin boot
	HFS_HFSP = 0xaf,           // HFS / HFS+
	BSDIfs = 0xb7,             // BSDI fs
	BSDIswap = 0xb8,           // BSDI swap
	BootWizardhid = 0xbb,      // Boot Wizard hid
	AcronisFAT32L = 0xbc,      // Acronis FAT32 L
	Solarisboot = 0xbe,        // Solaris boot
	Solaris = 0xbf,            // Solaris
	DRDOS_sec_FAT12 = 0xc1,    // DRDOS/sec (FAT-12
	DRDOS_sec_FAT16 = 0xc4,    // DRDOS/sec (FAT-16
	DRDOS_sec_FAT16B = 0xc6,   // DRDOS/sec (FAT-16B
	Syrinx = 0xc7,             // Syrinx
	ISO9660 = 0xcd,            // openSUSE ISOHybrid ISO9660
	NonFSdata = 0xda,          // Non-FS data
	CP_M_CTOS = 0xdb,          // CP/M / CTOS / .
	DellUtility = 0xde,        // Dell Utility
	BootIt = 0xdf,             // BootIt
	DOSaccess = 0xe1,          // DOS access
	DOSR_O = 0xe3,             // DOS R/O
	SpeedStorFAT16 = 0xe4,     // SpeedStor
	Linuxextended1 = 0xea,     // Linux extended
	BeOSfs = 0xeb,             // BeOS fs
	GPT = 0xee,                // GPT
	EFI_FAT_12_16 = 0xef,      // EFI (FAT-12/16/
	Linux_PA_RISCb = 0xf0,     // Linux/PA-RISC b
	SpeedStor0 = 0xf1,         // SpeedStor <- ?
	DOSsecondary = 0xf2,       // DOS secondary
	SpeedStorFAT16B = 0xf4,    // SpeedStor
	EBBRprotective = 0xf8,     // EBBR protective
	VMwareVMFS = 0xfb,         // VMware VMFS
	VMwareVMKCORE = 0xfc,      // VMware VMKCORE
	Linuxraidauto = 0xfd,      // Linux raid auto
	LANstep = 0xfe,            // LANstep
	BBT = 0xff,                // BBT
}

impl LowerHex for PartitionType {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		LowerHex::fmt(&(*self as u8), f)
	}
}

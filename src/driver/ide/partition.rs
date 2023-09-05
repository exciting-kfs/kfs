pub mod entry;

use core::{
	array,
	fmt::LowerHex,
	mem::MaybeUninit,
	ops::{Deref, DerefMut},
};

use alloc::boxed::Box;

use crate::{
	mm::{constant::SECTOR_SIZE, util::next_align},
	sync::locked::{Locked, LockedGuard},
};

use self::entry::{EntryIndex, MaybeEntry};

use super::{
	get_ide_controller,
	ide_id::{IdeId, NR_IDE_DEV},
	lba::LBA28,
};

pub const NR_PRIMARY: usize = 4;

// TODO logical partition?
#[derive(Debug)]
struct PartitionTable([Locked<MaybeEntry>; NR_PRIMARY]);

impl PartitionTable {
	const fn empty() -> Self {
		Self([
			Locked::new(MaybeEntry::empty()),
			Locked::new(MaybeEntry::empty()),
			Locked::new(MaybeEntry::empty()),
			Locked::new(MaybeEntry::empty()),
		])
	}

	fn new(entries: [MaybeEntry; 4]) -> Self {
		Self(array::from_fn(|i| Locked::new(entries[i].clone())))
	}
}

impl Deref for PartitionTable {
	type Target = [Locked<MaybeEntry>; 4];
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for PartitionTable {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

static mut PART_TABLE: [PartitionTable; NR_PRIMARY] = [
	PartitionTable::empty(),
	PartitionTable::empty(),
	PartitionTable::empty(),
	PartitionTable::empty(),
];

const BOOT_SECTOR_MAGIC: u16 = 0xaa55;
const BOOT_SECTOR_OFFSET: usize = 0x1fe / 2;
const PART_TABLE_OFFSET: usize = 0x1be / 2;

fn read_partition_table(dev: IdeId) -> Option<[MaybeEntry; 4]> {
	let ide = get_ide_controller(dev);

	let mut sector = Box::new_uninit_slice(1);
	ide.ata
		.read_sectors(unsafe { LBA28::new_unchecked(0) }, &mut sector);

	let sector = unsafe { sector.assume_init() };

	if sector[0][BOOT_SECTOR_OFFSET] != BOOT_SECTOR_MAGIC {
		return None;
	}

	let mut part_table: MaybeUninit<[MaybeEntry; 4]> = MaybeUninit::uninit();
	unsafe {
		part_table
			.as_mut_ptr()
			.cast::<u16>()
			.copy_from_nonoverlapping(&sector[0][PART_TABLE_OFFSET], 32)
	};

	Some(unsafe { part_table.assume_init() })
}

pub fn init(devices: [Option<IdeId>; NR_IDE_DEV]) {
	for dev in devices {
		if let Some(dev) = dev {
			if let Some(entries) = read_partition_table(dev) {
				unsafe { PART_TABLE[dev.index()] = PartitionTable::new(entries) }
			}
		}
	}
}

pub fn byte_to_sector_count(byte: usize) -> usize {
	next_align(byte, SECTOR_SIZE) / SECTOR_SIZE
}

// TODO hda1 => a: minor, 1: entry index
pub fn get_partition_entry<'a>(id: IdeId, ei: EntryIndex) -> LockedGuard<'a, MaybeEntry> {
	unsafe { PART_TABLE[id.index()][ei.index()].lock() }
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

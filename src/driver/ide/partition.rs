use core::{array, fmt::Display, mem::transmute};

use alloc::boxed::Box;

use crate::{
	driver::bus::ata::{AtaController, RawSector},
	pr_debug,
	sync::locked::Locked,
};

#[repr(C)]
#[derive(Debug)]
pub struct PartitionTableEntry {
	attribute: u8,
	begin_h: u8,
	begin_s: u8,
	begin_c: u8,
	partition_type: u8,
	last_h: u8,
	last_s: u8,
	last_c: u8,
	begin_lba: u32,
	sector_count: u32,
}

#[derive(Debug)]
pub struct PartitionTable([PartitionTableEntry; 4]);

impl Display for PartitionTable {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		for (i, e) in self.0.iter().enumerate() {
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
				chs_to_lba(e.last_c, e.last_h, e.last_s)
			)?;
			write!(f, "\tsector count: {:x}\n", e.sector_count)?;
		}
		Ok(())
	}
}

static PART_TABLE: Locked<[Option<Box<PartitionTable>>; 4]> = Locked::new([None, None, None, None]);
const BOOT_SECTOR_MAGIC: u16 = 0xaa55;
const BOOT_SECTOR_OFFSET: usize = 0x1fe / 2;
const PARTION_TABLE_OFFSET: usize = 0x1be / 2;

pub fn init(devices: [Option<&Locked<AtaController>>; 4]) {
	let mut sector = Box::new([RawSector::new([0; 256])]);
	let table = devices.iter().map(|dev| {
		let boot_sector = dev.and_then(|ata| {
			ata.lock().read_sectors(0, sector.as_mut());
			(sector[0][BOOT_SECTOR_OFFSET] == BOOT_SECTOR_MAGIC).then_some(sector.as_mut())
		});

		boot_sector.map(|sector| unsafe {
			let src = &sector[0][PARTION_TABLE_OFFSET..BOOT_SECTOR_OFFSET];
			let dst = array::from_fn(|i| src[i]);
			Box::new(transmute::<[u16; 32], _>(dst))
		})
	});

	table
		.enumerate()
		.for_each(|(i, e)| PART_TABLE.lock()[i] = e);
}

fn print_partition_table() {
	for (i, tab) in PART_TABLE.lock().iter().enumerate() {
		if let Some(t) = tab {
			pr_debug!("{}:\n{}", i, t);
		}
	}
}

fn chs_to_lba(c: u8, h: u8, s: u8) -> usize {
	// Hmm...
	const HPC: isize = 16;
	const SPT: isize = 63;

	let c = (s as isize & 0xc0 << 8) + c as isize;
	let s = s as isize & 0x3f;
	let h = h as isize;

	((c * HPC + h) * SPT + (s - 1)) as usize
}

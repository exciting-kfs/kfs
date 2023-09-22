pub mod bdf;
pub mod header;

use alloc::collections::BTreeMap;

use crate::{driver::bus::pci::header::HeaderCommon, pr_info, sync::Locked};

use self::bdf::BDF;

#[derive(Debug)]
pub enum Error {
	DeviceNotFound,
	UnexpectedHeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClassCode {
	pub class: u8,
	pub sub_class: u8,
}

static PCI_DEVICE: Locked<BTreeMap<ClassCode, BDF>> = Locked::new(BTreeMap::new());

pub fn enumerate() {
	pr_info!("INIT: PCI enumeration.");
	for bus in 0..=255 {
		for dev in 0..32 {
			for func in 0..8 {
				let bdf = BDF { bus, dev, func };

				if let Some(h) = HeaderCommon::get(&bdf) {
					let c = ClassCode {
						class: h.class,
						sub_class: h.sub_class,
					};
					PCI_DEVICE.lock().insert(c, bdf);
				}
			}
		}
	}
}

pub fn find_device(c: ClassCode) -> Result<BDF, Error> {
	let devices = PCI_DEVICE.lock();
	devices.get(&c).cloned().ok_or(Error::DeviceNotFound)
}

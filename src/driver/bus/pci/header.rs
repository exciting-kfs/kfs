use core::mem::{size_of, transmute};

use super::{Error, BDF};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct HeaderCommon {
	pub vendor_id: u16,
	pub device_id: u16,
	pub command: u16,
	pub status: u16,
	pub revision_id: u8,
	pub prog_if: u8,
	pub sub_class: u8,
	pub class: u8,
	pub cache_line_size: u8,
	pub latency_timer: u8,
	pub header_type: u8,
	pub bist: u8,
}

impl HeaderCommon {
	const BUF_LEN: usize = size_of::<HeaderCommon>() / size_of::<u32>();
	pub fn get(bdf: &BDF) -> Option<Self> {
		let mut buf: [u32; Self::BUF_LEN] = [0; Self::BUF_LEN];
		buf[0] = bdf.read_u32(0x0);
		if buf[0] == 0xffff_ffff {
			return None;
		}

		for i in 1..Self::BUF_LEN {
			let off = i * 4;
			buf[i] = bdf.read_u32(off as u8);
		}

		unsafe { Some(transmute(buf)) }
	}
}

#[repr(C)]
#[derive(Debug)]
pub struct HeaderType0 {
	pub common: HeaderCommon,
	pub bar0: u32,
	pub bar1: u32,
	pub bar2: u32,
	pub bar3: u32,
	pub bar4: u32,
	pub bar5: u32,
	pub cardbus_cis_ptr: u32,
	pub subsystem_vendor_id: u16,
	pub subsystem_id: u16,
	pub rom_base_address: u32,
	pub capabilities_ptr: u8,
	_reserve0: u8,
	_reserve1: u16,
	_reserve2: u32,
	pub irq_line: u8,
	pub irq_pin: u8,
	pub min_grant: u8,
	pub max_latency: u8,
}

impl HeaderType0 {
	const BUF_LEN: usize = (size_of::<HeaderType0>()) / size_of::<u32>();
	pub fn from_common(bdf: &BDF, common: HeaderCommon) -> Result<Self, Error> {
		if common.header_type != 0 {
			return Err(Error::UnexpectedHeader);
		}

		let common: [u32; HeaderCommon::BUF_LEN] = unsafe { transmute(common) };
		let mut buf: [u32; Self::BUF_LEN] = [0; Self::BUF_LEN];

		for i in HeaderCommon::BUF_LEN..Self::BUF_LEN {
			buf[i] = bdf.read_u32(i as u8 * 4);
		}

		for i in 0..HeaderCommon::BUF_LEN {
			buf[i] = common[i];
		}

		Ok(unsafe { transmute(buf) })
	}

	pub fn get(bdf: &BDF) -> Result<Self, Error> {
		let common = HeaderCommon::get(bdf).ok_or(Error::DeviceNotFound)?;
		HeaderType0::from_common(bdf, common)
	}
}

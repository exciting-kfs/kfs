use crate::pr_info;

use crate::io::pmio::Port;

static DATA: Port = Port::new(0xcfc);
static INDEX: Port = Port::new(0xcf8);

const ENABLE: u32 = 0x8000_0000;

fn set_index(bus: u8, device: u8, func: u8, offset: u8) {
	INDEX.write_u32(
		ENABLE | (bus as u32) << 16 | (device as u32) << 11 | (func as u32) << 8 | offset as u32,
	);
}

pub fn read_u32(bus: u8, device: u8, func: u8, offset: u8) -> u32 {
	set_index(bus, device, func, offset);
	DATA.read_u32()
}

pub fn write_u32(bus: u8, device: u8, func: u8, offset: u8, data: u32) {
	set_index(bus, device, func, offset);
	DATA.write_u32(data);
}

pub fn dump_pci() {
	for bus in 0..=255 {
		for device in 0..32 {
			for func in 0..8 {
				let id = read_u32(bus, device, func, 0);
				if id == 0xffff_ffff {
					continue;
				}

				let class = read_u32(bus, device, func, 0x8) >> 8;
				pr_info!("CLASS: {:#010x}", class);

				if class & !0xff == 0x00010100 {
					pr_info!("IDE CONTROLLER: B/D/F={}/{}/{}", bus, device, func);
				}
			}
		}
	}
}

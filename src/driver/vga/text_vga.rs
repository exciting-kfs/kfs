mod attr;
mod color;
mod screen_char;

pub use attr::Attr;
pub use color::Color;
pub use screen_char::Char;

use crate::io::pmio::Port;
use core::ptr;

pub const WIDTH: usize = 80;
pub const HEIGHT: usize = 25;
pub const WINDOW_SIZE: usize = WIDTH * HEIGHT;
const MMIO_ADDR: *mut Char = 0xb8000 as *mut Char; // TODO: use 2d array type

static INDEX_PORT: Port = Port::new(0x03d4);
static DATA_PORT: Port = Port::new(0x03d5);

pub fn put_slice_iter<'a, Iter>(collection: Iter)
where
	Iter: IntoIterator<Item = &'a [Char]>,
{
	let mut offset: isize = 0;

	for chunk in collection.into_iter() {
		let size = chunk.len();

		unsafe {
			MMIO_ADDR
				.offset(offset)
				.copy_from_nonoverlapping(chunk.as_ptr(), size as usize)
		};

		offset += size as isize;
	}
}

pub fn putc(y: usize, x: usize, c: Char) {
	if x >= WIDTH || y >= HEIGHT {
		panic!("putc: invalid coordinate ({y}, {x}), ");
	}
	unsafe { ptr::write_volatile(addr_of(y, x), c) }
}

pub fn clear() {
	let attr = Attr::new(false, Color::Black, false, Color::Black);
	let black = Char::styled(attr, b'\0');

	for y in 0..(HEIGHT) {
		for x in 0..(WIDTH) {
			putc(y, x, black);
		}
	}
}

pub fn enable_cursor(start: usize, end: usize) {
	INDEX_PORT.write_byte(0x0a); // cursor start
	let start = DATA_PORT.read_byte() & 0xc0 | start as u8;
	DATA_PORT.write_byte(start);

	INDEX_PORT.write_byte(0x0b); // cursor end
	let end = DATA_PORT.read_byte() & 0xe0 | end as u8;
	DATA_PORT.write_byte(end);
}

pub fn put_cursor(y: usize, x: usize) {
	let offset = offset_count(y, x);
	let low = offset & 0xff;
	let high = (offset >> 8) & 0xff;

	INDEX_PORT.write_byte(0x0f); // cursor position low
	DATA_PORT.write_byte(low as u8);

	INDEX_PORT.write_byte(0x0e); // cursor position high
	DATA_PORT.write_byte(high as u8);
}

fn addr_of(y: usize, x: usize) -> *mut Char {
	let count = offset_count(y, x);
	unsafe { MMIO_ADDR.offset(count as isize) }
}

fn offset_count(y: usize, x: usize) -> usize {
	y * WIDTH + x
}

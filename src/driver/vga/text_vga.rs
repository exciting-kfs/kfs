use crate::{driver::terminal::WinSize, io::pmio::Port, mm::util::phys_to_virt};
use core::ptr;

use super::{Attr, Char, Color, VGA};

const WIDTH: usize = 80;
const HEIGHT: usize = 25;
const WINDOW_SIZE: usize = WIDTH * HEIGHT;

const MMIO_ADDR: *mut Char = phys_to_virt(0xb8000) as *mut Char;

static INDEX_PORT: Port = Port::new(0x03d4);
static DATA_PORT: Port = Port::new(0x03d5);

pub struct TextVga;

impl VGA for TextVga {
	fn clear(&self) {
		let attr = Attr::new(Color::Black, Color::Black);
		let black = Char::styled(attr, b' ');

		for y in 0..(HEIGHT) {
			for x in 0..(WIDTH) {
				putc(y, x, black);
			}
		}
	}

	fn draw_text_buffer<'a, It>(&self, text_buffer: It)
	where
		It: Iterator<Item = &'a Char>,
	{
		// TODO
		for (i, ch) in text_buffer.enumerate().take(25 * 80) {
			unsafe { MMIO_ADDR.add(i).write(*ch) };
		}
	}

	fn draw_cursor(&self, y: usize, x: usize) {
		let offset = y * WIDTH + x;
		let low = offset & 0xff;
		let high = (offset >> 8) & 0xff;

		INDEX_PORT.write_byte(0x0f); // cursor position low
		DATA_PORT.write_byte(low as u8);

		INDEX_PORT.write_byte(0x0e); // cursor position high
		DATA_PORT.write_byte(high as u8);
	}

	fn draw_buffer(&self, _buffer: &[u32]) {}

	fn get_window_size(&self) -> WinSize {
		WinSize { row: 0, col: 0 }
	}

	fn get_text_window_size(&self) -> WinSize {
		WinSize {
			row: HEIGHT as u16,
			col: WIDTH as u16,
		}
	}
}

pub fn init() -> TextVga {
	enable_cursor(0, 11);

	TextVga
}

fn putc(y: usize, x: usize, c: Char) {
	if x >= WIDTH || y >= HEIGHT {
		panic!("putc: invalid coordinate ({y}, {x}), ");
	}
	unsafe { ptr::write_volatile(addr_of(y, x), c) }
}

fn enable_cursor(start: usize, end: usize) {
	INDEX_PORT.write_byte(0x0a); // cursor start
	let start = DATA_PORT.read_byte() & 0xc0 | start as u8;
	DATA_PORT.write_byte(start);

	INDEX_PORT.write_byte(0x0b); // cursor end
	let end = DATA_PORT.read_byte() & 0xe0 | end as u8;
	DATA_PORT.write_byte(end);
}

fn addr_of(y: usize, x: usize) -> *mut Char {
	let count = offset_count(y, x);
	unsafe { MMIO_ADDR.offset(count as isize) }
}

fn offset_count(y: usize, x: usize) -> usize {
	y * WIDTH + x
}

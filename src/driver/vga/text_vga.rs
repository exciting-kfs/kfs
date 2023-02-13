mod attr;
mod color;
mod screen_char;

pub use attr::Attr;
pub use color::Color;
pub use screen_char::Char;

use crate::console;
use crate::raw_io::pmio::Port;
use core::ptr;

pub const WIDTH: usize = 80;
pub const HEIGHT: usize = 25;
const MMIO_ADDR: *mut u16 = 0xb8000 as *mut u16; // TODO: use 2d array type

pub fn draw(buf: &[[Char; WIDTH]; console::BUFFER_HEIGHT], mut buf_y: usize) {
	let mut vga_y = 0;

	while buf_y < console::BUFFER_HEIGHT && vga_y < HEIGHT {
		put_line(vga_y, &buf[buf_y]);
		buf_y += 1;
		vga_y += 1;
	}

	buf_y = 0;
	while vga_y < HEIGHT {
		put_line(vga_y, &buf[buf_y]);
		vga_y += 1;
	}
}

pub fn put_line(y: usize, line: &[Char; WIDTH]) {
	for x in 0..WIDTH {
		putc(y, x, line[x]);
	}
}

pub fn putc(y: usize, x: usize, c: Char) {
	if x >= WIDTH || y >= HEIGHT {
		panic!("putc: invalid coordinate ({y}, {x}), ");
	}
	unsafe { ptr::write_volatile(addr_of(y, x), c.0) }
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

pub fn put_cursor(y: usize, x: usize) {
	let offset = offset_count(y, x);
	let low = offset & 0xff;
	let high = (offset >> 8) & 0xff;

	let reg_select = Port::new(0x03d4);
	let reg_data = Port::new(0x03d5);

	reg_select.write_byte(0x0f); // cursor position low
	reg_data.write_byte(low as u8);

	reg_select.write_byte(0x0e); // cursor position high
	reg_data.write_byte(high as u8);

	// unsafe {
	// 	asm!( 	// bx = x * width + y

	// 		"mov dx, 0x03D4",	// pointer index register
	// 		"mov al, 0x0F",		// select cursor low
	// 		"out dx, al",

	// 		"inc dl",		// pointer data register
	// 		"mov al, bl",		// write bl ?
	// 		"out dx, al",

	// 		"dec dl",		// dx = 0x03d4
	// 		"mov al, 0x0E",		// ?
	// 		"out dx, al",

	// 		"inc dl",		// dx = 0x03d5
	// 		"mov al, bh",		// write bh ?
	// 		"out dx, al",
	// 	)
	// }
}

fn addr_of(y: usize, x: usize) -> *mut u16 {
	let count = offset_count(y, x);
	unsafe { MMIO_ADDR.offset(count as isize) }
}

fn offset_count(y: usize, x: usize) -> usize {
	y * WIDTH + x
}

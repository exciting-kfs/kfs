use core::{ptr::NonNull, slice::from_raw_parts_mut};

use crate::{
	driver::terminal::WinSize,
	mm::{alloc::virt::io_allocate, constant::PAGE_SIZE},
	syscall::errno::Errno,
};

const FONT: &[u8] = include_bytes!("VGA8.f16");

use super::{Char, FrameBufferInfo, FRAME_BUFFER_INFO, VGA};

const TEXT_WIDTH: usize = 8;
const TEXT_HEIGHT: usize = 16;
const TEXT_SIZE: usize = TEXT_WIDTH * TEXT_HEIGHT;

const WIDTH: usize = 1280;
const HEIGHT: usize = 800;

const TEXT_PER_COL: usize = WIDTH / TEXT_WIDTH;
const TEXT_PER_ROW: usize = HEIGHT / TEXT_HEIGHT;

pub struct FrameBuffer {
	base: NonNull<u32>,
}

const COLOR_MAPPER: [u32; 16] = [
	0x000000, 0x0000AA, 0x00AA00, 0x00AAAA, 0xAA0000, 0xAA00AA, 0xAA5500, 0xAAAAAA, 0x555555,
	0x5555FF, 0x55FF55, 0x55FFFF, 0xFF5555, 0xFF55FF, 0xFFFF55, 0xFFFFFF,
];

impl VGA for FrameBuffer {
	fn clear(&self) {
		for x in self.as_buf() {
			*x = 0;
		}
	}

	fn draw_text_buffer<'a, It>(&self, mut text_buffer: It)
	where
		It: Iterator<Item = &'a Char>,
	{
		let vga_buffer = self.as_buf();

		for y in 0..TEXT_PER_ROW {
			for x in 0..TEXT_PER_COL {
				let (ch, style) = match text_buffer.next() {
					Some(x) => (x.into_u8(), x.get_attr()),
					None => return,
				};

				let font_base = ch as usize * 16;
				let font = &FONT[font_base..font_base + 16];

				for yoffset in 0..TEXT_HEIGHT {
					for xoffset in 0..TEXT_WIDTH {
						let ybase = y * TEXT_HEIGHT;
						let xbase = x * TEXT_WIDTH;

						vga_buffer[((ybase + yoffset) * WIDTH) + (xbase + xoffset)] =
							match (font[yoffset] & (0b1000_0000 >> xoffset)) == 0 {
								true => COLOR_MAPPER[style.get_bg() as usize],
								false => COLOR_MAPPER[style.get_fg() as usize],
							};
					}
				}
			}
		}
	}

	fn draw_cursor(&self, y: usize, x: usize) {
		let base = y * WIDTH * TEXT_HEIGHT + x * TEXT_WIDTH;

		for y in 2..(TEXT_HEIGHT - 2) {
			for x in 1..(TEXT_WIDTH - 1) {
				self.as_buf()[base + y * WIDTH + x] = 0xaaaaaa;
			}
		}
	}

	fn draw_buffer(&self, buffer: &[u32]) {
		let vga_buffer = self.as_buf();
		let size = buffer.len().min(vga_buffer.len());

		vga_buffer[..size].copy_from_slice(&buffer[..size]);
	}

	fn get_text_window_size(&self) -> WinSize {
		WinSize {
			row: (HEIGHT / TEXT_HEIGHT) as u16,
			col: (WIDTH / TEXT_WIDTH) as u16,
		}
	}

	fn get_window_size(&self) -> WinSize {
		WinSize {
			row: HEIGHT as u16,
			col: WIDTH as u16,
		}
	}
}

impl FrameBuffer {
	fn as_buf(&self) -> &mut [u32] {
		unsafe { from_raw_parts_mut(self.base.as_ptr(), WIDTH * HEIGHT) }
	}

	fn new(info: &FrameBufferInfo) -> Result<Self, Errno> {
		if info.height as usize != HEIGHT || info.width as usize != WIDTH || info.bpp != 32 {
			return Err(Errno::EINVAL);
		}

		let base = io_allocate(info.address, WIDTH * HEIGHT * 4 / PAGE_SIZE)
			.map(|mut x| unsafe { NonNull::new_unchecked(x.as_mut().as_mut_ptr().cast::<u32>()) })
			.map_err(|_| Errno::ENOMEM)?;

		Ok(Self { base })
	}
}

pub fn init() -> Result<FrameBuffer, Errno> {
	match &*FRAME_BUFFER_INFO.lock() {
		Some(fb) => FrameBuffer::new(fb),
		None => Err(Errno::ENOSYS),
	}
}

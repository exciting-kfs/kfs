mod attr;
mod color;
mod screen_char;

mod framebuffer;
mod text_vga;

pub use attr::*;
pub use color::*;
pub use screen_char::*;

use core::{mem::MaybeUninit, slice::from_raw_parts};

use enum_dispatch::enum_dispatch;

use crate::{
	mm::user::{verify::verify_region, vma::AreaFlag},
	process::task::CURRENT,
	sync::Locked,
	syscall::errno::Errno,
};

use self::{framebuffer::FrameBuffer, text_vga::TextVga};

use super::terminal::WinSize;

pub struct FrameBufferInfo {
	pub address: usize,
	pub width: u32,
	pub height: u32,
	pub bpp: u8,
}

impl From<multiboot2::FramebufferTag<'_>> for FrameBufferInfo {
	fn from(value: multiboot2::FramebufferTag) -> Self {
		Self {
			address: value.address as usize,
			width: value.width,
			height: value.height,
			bpp: value.bpp,
		}
	}
}

pub static FRAME_BUFFER_INFO: Locked<Option<FrameBufferInfo>> = Locked::new(None);

static mut SELECTED_VGA: MaybeUninit<Vga> = MaybeUninit::uninit();

#[enum_dispatch]
pub trait VGA {
	fn clear(&self);

	fn draw_text_buffer<'a, It>(&self, text_buffer: It)
	where
		It: Iterator<Item = &'a Char>;
	fn draw_cursor(&self, y: usize, x: usize);
	fn get_text_window_size(&self) -> WinSize;

	fn draw_buffer(&self, buffer: &[u32]);
	fn get_window_size(&self) -> WinSize;
}

#[enum_dispatch(VGA)]
enum Vga {
	Text(TextVga),
	FrameBuffer(FrameBuffer),
}

pub fn init() {
	let vga_backend = framebuffer::init()
		.map(|fb| Vga::FrameBuffer(fb))
		.unwrap_or_else(|_| Vga::Text(text_vga::init()));

	unsafe { SELECTED_VGA.write(vga_backend) };

	clear();
}

fn get_vga_backend() -> &'static Vga {
	unsafe { SELECTED_VGA.assume_init_ref() }
}

pub fn clear() {
	get_vga_backend().clear();
}

pub fn draw_text_buffer<'a, It>(text_buffer: It)
where
	It: Iterator<Item = &'a Char>,
{
	get_vga_backend().draw_text_buffer(text_buffer);
}

pub fn draw_cursor(y: usize, x: usize) {
	get_vga_backend().draw_cursor(y, x);
}

pub fn draw_buffer(buffer: &[u32]) {
	get_vga_backend().draw_buffer(buffer);
}

pub fn get_window_size() -> WinSize {
	get_vga_backend().get_window_size()
}

pub fn get_text_window_size() -> WinSize {
	get_vga_backend().get_text_window_size()
}

pub fn sys_draw_buffer(buffer: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let winsize = get_window_size();
	let winsize = winsize.col as usize * winsize.row as usize;

	verify_region(buffer, 4 * winsize, current, AreaFlag::Readable)?;

	let buf = unsafe { from_raw_parts(buffer as *const u32, winsize) };

	draw_buffer(buf);

	Ok(0)
}

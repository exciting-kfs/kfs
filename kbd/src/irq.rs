use core::arch::global_asm;

use alloc::sync::Arc;
use kernel::driver::apic::io::{IO_APIC, KEYBOARD_IRQ};
use kernel::elf::kobject::KernelModule;
use kernel::input::key_event::KeyEvent;
use kernel::input::keyboard::{KbdDriver, KEYBOARD};
use kernel::interrupt::idt::register_irq;
use kernel::syscall::errno::Errno;
use kernel::x86::{SystemDesc, DPL_USER, GDT};

struct Ps2Kbd(Arc<KernelModule>);

impl KbdDriver for Ps2Kbd {
	fn get_key_event(&self) -> Option<KeyEvent> {
		crate::ps2::keyboard::get_key_event()
	}

	fn reset_cpu(&self) {
		crate::ps2::control::reset_cpu();
	}
}

extern "C" {
	fn handle_keyboard();
}

global_asm!(
	".global handle_keyboard",
	"handle_keyboard:",
	"push %eax",
	"lea handle_keyboard_impl, %eax",
	"push %eax",
	"movl 4(%esp), %eax",
	"movl $0, 4(%esp)",
	"jmp handle_interrupt",
	options(att_syntax),
);

pub fn init(module: Arc<KernelModule>) -> Result<(), Errno> {
	let keyboard = SystemDesc::new_interrupt(handle_keyboard as usize, GDT::KERNEL_CODE, DPL_USER);

	register_irq(0x21, keyboard);

	let driver = Arc::new(Ps2Kbd(module));
	unsafe { _ = KEYBOARD.attach(driver) };

	let mut apic = IO_APIC.lock();
	let apic = unsafe { apic.assume_init_mut() };

	let mut kbd = apic.read_redir(KEYBOARD_IRQ).unwrap();

	kbd.set_mask(false);

	apic.write_redir(KEYBOARD_IRQ, kbd).unwrap();

	Ok(())
}

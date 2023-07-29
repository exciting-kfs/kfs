use core::{cell::UnsafeCell, mem::MaybeUninit};

use kfs_macro::context;

use crate::{config::NR_CPUS, smp::smp_id};

pub struct CpuLocal<T> {
	data: UnsafeCell<MaybeUninit<[T; NR_CPUS]>>,
}

unsafe impl<T> Sync for CpuLocal<T> {}

impl<T> CpuLocal<T> {
	pub const fn uninit() -> Self {
		Self {
			data: UnsafeCell::new(MaybeUninit::uninit()),
		}
	}

	pub const fn new(value: T) -> Self
	where
		T: Copy,
	{
		Self {
			data: UnsafeCell::new(MaybeUninit::new([value; NR_CPUS])),
		}
	}

	pub fn init(&self, value: T) {
		unsafe { self.data.get().cast::<T>().add(smp_id()).write(value) };
	}

	/// precondition: context(irq_disabled).
	pub unsafe fn get_mut(&self) -> &mut T {
		let arr = self.arr_mut();

		&mut arr[smp_id()]
	}

	#[context(irq_disabled)]
	pub unsafe fn replace(&self, src: T) -> T {
		let arr = self.arr_mut();
		let dest = &mut arr[smp_id()];
		core::mem::replace(dest, src)
	}

	fn arr_mut<'l>(&self) -> &'l mut [T; NR_CPUS] {
		unsafe { self.data.get().as_mut::<'l>().unwrap().assume_init_mut() }
	}
}

#[cfg(disable)]
mod test {
	use crate::pr_info;
	use kfs_macro::ktest;

	use super::*;

	#[derive(Debug)]
	struct A {
		a: usize,
		b: usize,
	}

	static AA: CpuLocal<A> = CpuLocal::zeroed();

	#[ktest(dev)]
	fn test() {
		let mut a = AA.get_mut();
		let mut b = AA.get_mut();

		b.a = 2;
		a.a = 1;

		let c = AA.get_mut();

		pr_info!("c.a: {}", c.a);
		pr_info!("c.b: {}", c.b);
	}
}

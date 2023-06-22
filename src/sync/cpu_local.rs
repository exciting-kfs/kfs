use core::{
	cell::UnsafeCell,
	mem::MaybeUninit,
	ops::{Deref, DerefMut},
};

use crate::{
	config::NR_CPUS,
	interrupt::{irq_stack_restore, irq_stack_save},
	smp::smp_id,
};

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

	pub fn init(&self, value: T) {
		unsafe { self.data.get().cast::<T>().add(smp_id()).write(value) };
	}

	pub fn get_mut(&self) -> LocalValue<'_, T> {
		let arr = self.arr_mut();

		LocalValue::new(&mut arr[smp_id()])
	}

	pub fn replace(&self, src: T) -> T {
		let arr = self.arr_mut();
		let dest = &mut arr[smp_id()];
		core::mem::replace(dest, src)
	}

	fn arr_mut<'l>(&self) -> &'l mut [T; NR_CPUS] {
		unsafe { self.data.get().as_mut::<'l>().unwrap().assume_init_mut() }
	}
}

pub struct LocalValue<'l, T> {
	value: &'l mut T,
}

impl<'l, T> LocalValue<'l, T> {
	fn new(value: &'l mut T) -> Self {
		irq_stack_save();
		LocalValue { value }
	}
}

impl<'l, T> Deref for LocalValue<'l, T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		self.value
	}
}

impl<'l, T> DerefMut for LocalValue<'l, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.value
	}
}

impl<'l, T> Drop for LocalValue<'l, T> {
	fn drop(&mut self) {
		irq_stack_restore();
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

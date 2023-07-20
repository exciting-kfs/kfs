//! Basic character-by-character I/O

/// readable object
pub trait Read<T> {
	fn read_one(&mut self) -> Option<T>;
}

/// writable object
pub trait Write<T> {
	fn write_one(&mut self, data: T) -> Result<(), super::NoSpace>;
}

/// readable and writeable.
/// you can think this trait as from type `I` to type `O` converter
pub trait RW<I, O>: Write<I> + Read<O> {}

// chain two different RW object.
// input(`I`) -> obj(`src`) -> intermediate(`M`) -> output(`O`)
// so chain is also RW<I, O>
// pub struct Chain<'a, I, M, O> {
// 	src: &'a mut dyn RW<I, M>,
// 	dst: &'a mut dyn RW<M, O>,
// 	max_repeat: usize,
// }

// impl<'a, I, M, O> Chain<'a, I, M, O> {
// 	pub fn new(src: &'a mut dyn RW<I, M>, dst: &'a mut dyn RW<M, O>) -> Self {
// 		Self {
// 			src,
// 			dst,
// 			max_repeat: 10,
// 		}
// 	}
// }

// impl<'a, I, M, O> Read<O> for Chain<'a, I, M, O> {
// 	fn read_one(&mut self) -> Option<O> {
// 		self.dst.read_one()
// 	}
// }

// impl<'a, I, M, O> Write<I> for Chain<'a, I, M, O> {
// 	fn write_one(&mut self, data: I) -> bool {
// 		self.src.write_one(data);

// 		for _ in 0..self.max_repeat {
// 			match self.src.read_one() {
// 				Some(v) => self.dst.write_one(v),
// 				None => return,
// 			}
// 		}
// 	}
// }

// impl<'a, I, M, O> RW<I, O> for Chain<'a, I, M, O> {}

use core::slice;

use alloc::vec::Vec;

#[derive(Debug)]
pub struct Path {
	base: Base,
	comps: Vec<Vec<u8>>,
}

#[derive(Debug, PartialEq)]
pub enum Component<'a> {
	ParentDir,
	CurDir,
	Part(&'a [u8]),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Base {
	RootDir,
	WorkingDir { to_parent: usize },
}

impl Base {
	pub fn move_to_parent_dir(&mut self) {
		use Base::*;
		if let WorkingDir { to_parent } = self {
			*to_parent += 1;
		}
	}
}

pub struct ComponentIter<'a> {
	path: &'a [u8],
}

impl<'a> ComponentIter<'a> {
	fn new(path: &'a [u8]) -> Self {
		let mut it = Self { path };

		it.ignore_slash();

		it
	}

	fn ignore_slash(&mut self) {
		let slash_end = self
			.path
			.iter()
			.position(|ch| *ch != b'/')
			.unwrap_or(self.path.len());

		let (_, next) = self.path.split_at(slash_end);

		self.path = next;
	}

	fn take_before_slash(&mut self) -> &'a [u8] {
		let slash_start = self
			.path
			.iter()
			.position(|ch| *ch == b'/')
			.unwrap_or(self.path.len());

		let (comp, next) = self.path.split_at(slash_start);

		self.path = next;

		comp
	}
}

impl<'a> Iterator for ComponentIter<'a> {
	type Item = Component<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.path.len() == 0 {
			return None;
		}

		let comp = self.take_before_slash();
		self.ignore_slash();

		let comp = if comp == b"." {
			Component::CurDir
		} else if comp == b".." {
			Component::ParentDir
		} else {
			Component::Part(comp)
		};

		Some(comp)
	}
}

impl Path {
	pub fn new(path: &[u8]) -> Self {
		let mut base = if path.len() != 0 && path[0] == b'/' {
			Base::RootDir
		} else {
			Base::WorkingDir { to_parent: 0 }
		};

		let raw_comps = ComponentIter::new(path);
		let mut comps = Vec::new();

		for comp in raw_comps {
			use Component::*;
			match comp {
				ParentDir => match comps.is_empty() {
					true => base.move_to_parent_dir(),
					false => _ = comps.pop(),
				},
				Part(p) => comps.push(p.to_vec()),
				CurDir => (),
			}
		}

		Self { base, comps }
	}

	pub fn base(&self) -> Base {
		self.base
	}

	pub fn pop_component(&mut self) -> Option<Vec<u8>> {
		self.comps.pop()
	}

	pub fn components(&self) -> slice::Iter<'_, Vec<u8>> {
		self.comps.iter()
	}
}

mod test {
	use super::*;
	use kfs_macro::ktest;

	#[ktest(path)]
	fn empty() {
		let p = Path::new(b"");

		assert_eq!(p.base(), Base::WorkingDir { to_parent: 0 });
		assert_eq!(p.components().next(), None);
	}

	#[ktest(path)]
	fn basic() {
		let path = Path::new(b"abcd");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::WorkingDir { to_parent: 0 });
		assert_eq!(comps.next().unwrap(), b"abcd");
		assert_eq!(comps.next(), None);

		let path = Path::new(b"/abcd");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::RootDir);
		assert_eq!(comps.next().unwrap(), b"abcd");
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn trailing_slash() {
		let path = Path::new(b"abcd/");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::WorkingDir { to_parent: 0 });
		assert_eq!(comps.next().unwrap(), b"abcd");
		assert_eq!(comps.next(), None);

		let path = Path::new(b"/abcd/");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::RootDir);
		assert_eq!(comps.next().unwrap(), b"abcd");
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn multiple_slash() {
		let path = Path::new(b"abcd////");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::WorkingDir { to_parent: 0 });
		assert_eq!(comps.next().unwrap(), b"abcd");
		assert_eq!(comps.next(), None);

		let path = Path::new(b"////abcd");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::RootDir);
		assert_eq!(comps.next().unwrap(), b"abcd");
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn curdir() {
		let path = Path::new(b"./abcd");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::WorkingDir { to_parent: 0 });
		assert_eq!(comps.next().unwrap(), b"abcd");
		assert_eq!(comps.next(), None);

		let path = Path::new(b"/./abcd");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::RootDir);
		assert_eq!(comps.next().unwrap(), b"abcd");
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn parentdir() {
		let path = Path::new(b"../abcd");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::WorkingDir { to_parent: 1 });
		assert_eq!(comps.next().unwrap(), b"abcd");
		assert_eq!(comps.next(), None);

		let path = Path::new(b"/../abcd");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::RootDir);
		assert_eq!(comps.next().unwrap(), b"abcd");
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn single_component() {
		let path = Path::new(b"..");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::WorkingDir { to_parent: 1 });
		assert_eq!(comps.next(), None);

		let path = Path::new(b".");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::WorkingDir { to_parent: 0 });
		assert_eq!(comps.next(), None);

		let path = Path::new(b"/");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::RootDir);
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn complex() {
		let path = Path::new(b"/./..//abc///././/../dddd//eeeeee//");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::RootDir);
		assert_eq!(comps.next().unwrap(), b"dddd");
		assert_eq!(comps.next().unwrap(), b"eeeeee");
		assert_eq!(comps.next(), None);

		let path = Path::new(b"./..//abc///././/../dddd//eeeeee//");

		let mut comps = path.components();
		assert_eq!(path.base(), Base::WorkingDir { to_parent: 1 });
		assert_eq!(comps.next().unwrap(), b"dddd");
		assert_eq!(comps.next().unwrap(), b"eeeeee");
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn pop_comps() {
		let mut path = Path::new(b"a/../../..");

		assert_eq!(path.base(), Base::WorkingDir { to_parent: 2 });
		assert_eq!(path.pop_component(), None);

		let mut path = Path::new(b"a/b/c/d/..");
		assert_eq!(path.base(), Base::WorkingDir { to_parent: 0 });
		assert_eq!(path.pop_component().unwrap(), b"c");
		assert_eq!(path.pop_component().unwrap(), b"b");
		assert_eq!(path.pop_component().unwrap(), b"a");
		assert_eq!(path.pop_component(), None);
	}
}

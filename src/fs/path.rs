use core::fmt::{self, Display};

use alloc::{
	collections::{vec_deque, VecDeque},
	vec::Vec,
};

macro_rules! format_path {
	($($arg:tt)*) => {
		Path::new(format!($($arg)*).as_bytes())
	};
}

pub(crate) use format_path;

#[derive(Debug, Clone)]
pub struct Path {
	base: Base,
	comps: VecDeque<Vec<u8>>,
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
	pub fn new_root() -> Self {
		Self {
			base: Base::RootDir,
			comps: VecDeque::new(),
		}
	}

	pub fn new(path: &[u8]) -> Self {
		let mut base = if path.len() != 0 && path[0] == b'/' {
			Base::RootDir
		} else {
			Base::WorkingDir { to_parent: 0 }
		};

		let raw_comps = ComponentIter::new(path);
		let mut comps = VecDeque::new();

		for comp in raw_comps {
			use Component::*;
			match comp {
				ParentDir => match comps.is_empty() {
					true => base.move_to_parent_dir(),
					false => _ = comps.pop_back(),
				},
				Part(p) => comps.push_back(p.to_vec()),
				CurDir => (),
			}
		}

		Self { base, comps }
	}

	pub fn base(&self) -> Base {
		self.base
	}

	pub fn pop_component(&mut self) -> Option<Vec<u8>> {
		self.comps.pop_back()
	}

	pub fn push_component_front(&mut self, comp: Vec<u8>) {
		if !comp.is_empty() {
			self.comps.push_front(comp);
		}
	}

	pub fn components(&self) -> vec_deque::Iter<'_, Vec<u8>> {
		self.comps.iter()
	}

	pub fn to_buffer(&self) -> Vec<u8> {
		let mut buf: Vec<u8> = Vec::new();

		use Base::*;
		if let WorkingDir { to_parent } = self.base() {
			{
				if to_parent == 0 {
					buf.push(b'.');
				} else {
					for ch in core::iter::repeat(&b".."[..])
						.take(to_parent)
						.intersperse(&b"/"[..])
						.flatten()
					{
						buf.push(*ch);
					}
				}
			}
		} else {
			buf.push(b'/');
		}

		for comp in self.components().map(|x| &x[..]).intersperse(&b"/"[..]) {
			for ch in comp {
				buf.push(*ch);
			}
		}

		buf
	}
}

impl Display for Path {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		use core::str;

		write!(
			f,
			"{}",
			str::from_utf8(&self.to_buffer()).unwrap_or("[unknown]")
		)
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

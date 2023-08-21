use alloc::vec::Vec;

pub struct Path(Vec<u8>);

#[derive(Debug, PartialEq)]
pub enum Component<'a> {
	RootDir,
	ParentDir,
	CurDir,
	Dir(&'a [u8]),
}

pub struct ComponentIter<'a> {
	path: &'a [u8],
	is_begining: bool,
}

impl<'a> ComponentIter<'a> {
	fn ignore_slash(&mut self) {
		let slash_end = self
			.path
			.iter()
			.enumerate()
			.find_map(|(i, ch)| (*ch != b'/').then_some(i))
			.unwrap_or(self.path.len());

		let (_, next) = self.path.split_at(slash_end);

		self.path = next;
	}

	fn take_before_slash(&mut self) -> &'a [u8] {
		let slash_start = self
			.path
			.iter()
			.enumerate()
			.find_map(|(i, ch)| (*ch == b'/').then_some(i))
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

		if self.is_begining {
			self.is_begining = false;
			let ret = match self.path[0] {
				b'/' => Component::RootDir,
				_ => Component::CurDir,
			};

			self.ignore_slash();

			return Some(ret);
		}

		let comp = self.take_before_slash();
		self.ignore_slash();

		let comp = if comp == b"." {
			Component::CurDir
		} else if comp == b".." {
			Component::ParentDir
		} else {
			Component::Dir(comp)
		};

		Some(comp)
	}
}

impl Path {
	pub fn new(bytes: &[u8]) -> Self {
		Path(Vec::from(bytes))
	}

	pub fn components(&self) -> ComponentIter<'_> {
		ComponentIter {
			path: &self.0,
			is_begining: true,
		}
	}
}

mod test {
	use super::*;
	use kfs_macro::ktest;
	use Component::*;

	#[ktest(path)]
	fn basic() {
		let path = Path::new(b"abcd");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(Dir(b"abcd")));
		assert_eq!(comps.next(), None);

		let path = Path::new(b"/abcd");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(RootDir));
		assert_eq!(comps.next(), Some(Dir(b"abcd")));
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn trailing_slash() {
		let path = Path::new(b"abcd/");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(Dir(b"abcd")));
		assert_eq!(comps.next(), None);

		let path = Path::new(b"/abcd/");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(RootDir));
		assert_eq!(comps.next(), Some(Dir(b"abcd")));
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn multiple_slash() {
		let path = Path::new(b"abcd////");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(Dir(b"abcd")));
		assert_eq!(comps.next(), None);

		let path = Path::new(b"////abcd");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(RootDir));
		assert_eq!(comps.next(), Some(Dir(b"abcd")));
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn curdir() {
		let path = Path::new(b"./abcd");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(Dir(b"abcd")));
		assert_eq!(comps.next(), None);

		let path = Path::new(b"/./abcd");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(RootDir));
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(Dir(b"abcd")));
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn parentdir() {
		let path = Path::new(b"../abcd");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(ParentDir));
		assert_eq!(comps.next(), Some(Dir(b"abcd")));
		assert_eq!(comps.next(), None);

		let path = Path::new(b"/../abcd");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(RootDir));
		assert_eq!(comps.next(), Some(ParentDir));
		assert_eq!(comps.next(), Some(Dir(b"abcd")));
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn single_component() {
		let path = Path::new(b"..");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(ParentDir));
		assert_eq!(comps.next(), None);

		let path = Path::new(b".");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), None);

		let path = Path::new(b"/");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(RootDir));
		assert_eq!(comps.next(), None);
	}

	#[ktest(path)]
	fn complex() {
		let path = Path::new(b"/./..//abc///././/../dddd//eeeeee//");

		let mut comps = path.components();
		assert_eq!(comps.next(), Some(RootDir));
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(ParentDir));
		assert_eq!(comps.next(), Some(Dir(b"abc")));
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(CurDir));
		assert_eq!(comps.next(), Some(ParentDir));
		assert_eq!(comps.next(), Some(Dir(b"dddd")));
		assert_eq!(comps.next(), Some(Dir(b"eeeeee")));
		assert_eq!(comps.next(), None);
	}
}

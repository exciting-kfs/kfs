mod free_node;

use core::ptr::NonNull;
use core::fmt::Debug;

pub use self::free_node::FreeNode;

pub struct FreeList {
	head: Option<NonNull<FreeNode>>,
}

impl FreeList {

	pub const fn new() -> Self {
		FreeList { head: None }
	}

	pub fn count(&self) -> usize {
		self.iter().count()
	}

	pub fn first(&self) -> Option<NonNull<FreeNode>> {
		self.head
	}

	pub fn last(&mut self) -> Option<NonNull<FreeNode>> {
		let head = unsafe { self.head.as_ref()?.as_ref() };
		Some(head.prev)
	}

	pub fn find_if<F>(&mut self, mut f: F) -> Option<NonNull<FreeNode>>
	where F: FnMut(&FreeNode) -> bool
	{
		self.iter_mut().find(|n| f(n)).map(|n| n.as_non_null())
	}

	pub fn check_double_free<T>(&mut self, ptr: NonNull<T>) -> bool
	{
		self.iter_mut().find(|node| node.contains(ptr)).is_some()
	}

	pub fn insert(&mut self, node: &mut FreeNode) {
		if let None = self.head {
			self.head = Some(node.as_non_null());
			return;
		}

		let base = unsafe { match self.find_if(|n| n.addr() > node.addr()) {
			Some(mut bp) =>  bp.as_mut(),
			None => self.head.unwrap().as_mut()
		}};
		
		self.insert_front(base, node);
		node.try_merge();
	}

	fn insert_front(&mut self, base: &mut FreeNode, node: &mut FreeNode) {
		let prev = unsafe { base.prev.as_mut() };
		let node_ptr = node.as_non_null();
		let base_ptr = base.as_non_null();

		prev.next = node_ptr;
		base.prev = node_ptr;

		node.next = base_ptr;
		node.prev = prev.as_non_null();

		let head = unsafe { self.head.unwrap().as_mut() };
		if base == head && node_ptr < base_ptr {
			self.head = Some(node_ptr);
		}
	}

	pub fn iter_mut(&mut self) -> IterMut<'_> {
		IterMut::new(self)
	}

	pub fn iter(&self) -> Iter<'_> {
		Iter::new(self)
	}

	pub fn remove_if<'page, F>(&mut self, f: F) -> Option<&'page mut FreeNode>
	where F: FnMut(&FreeNode) -> bool
	{
		self.find_if(f).map(|mut node_ptr|{
			let node = unsafe { node_ptr.as_mut() };
			self.remove(node);
			node
		})
	}

	fn remove(&mut self, node: &mut FreeNode) {
		self.head.map(|mut head_ptr| {
			let head = unsafe { head_ptr.as_mut()};
			if node == head {
				self.head = Some(head.next);
			}

			if head_ptr == head.next {
				self.head = None;
			}

			node.disjoint()
		});
	}
}

impl Debug for FreeList {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_list().entries(self).finish()
	}
}

impl<'a> Extend<&'a mut FreeNode> for FreeList {
	fn extend<T: IntoIterator<Item = &'a mut FreeNode>>(&mut self, iter: T) {
		iter.into_iter().for_each(|n| {
			n.disjoint();
			self.insert(n);
		})
	}
}

impl Default for FreeList {
	fn default() -> Self {
		FreeList::new()
	}
}

	/// Iterator - IterMut

impl<'iter> IntoIterator for &'iter mut FreeList {
	type Item = &'iter mut FreeNode;
	type IntoIter = IterMut<'iter>;
	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}

#[derive(Debug)]
pub struct IterMut<'iter> {
	head: Option<&'iter mut FreeNode>,
	curr: NonNull<FreeNode>,
}

impl<'iter> IterMut<'iter> {
	fn new(cont: &mut FreeList) -> Self {
		let (head, curr) = match cont.head {
			None => (None, NonNull::dangling()),
			Some(mut node) => (
				Some(&mut *unsafe { node.as_mut() }),
				node
			)
		};

		IterMut {
			head,
			curr,
		}
	}
}

impl<'iter> Iterator for IterMut<'iter> {
	type Item = &'iter mut FreeNode;
	fn next(&mut self) -> Option<Self::Item> {
		let head = self.head.as_ref()?;
		let curr = unsafe { self.curr.as_mut() };
		let next = unsafe { curr.next.as_mut() };
		
		if next == *head || curr == next { // for partition
			self.head = None;
		}

		self.curr = curr.next;
		Some(curr)
	}
}

/// Iterator - Iter

impl<'iter> IntoIterator for &'iter FreeList {
	type Item = &'iter FreeNode;
	type IntoIter = Iter<'iter>;
	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

#[derive(Debug)]
pub struct Iter<'iter> {
	head: Option<&'iter FreeNode>,
	curr: NonNull<FreeNode>
}

impl<'iter> Iter<'iter> {
	fn new(cont: &FreeList) -> Self {
		let (head, curr) = match cont.head {
			None => (None, NonNull::dangling()),
			Some(node) => (
				Some(&* unsafe { node.as_ref() }),
				node
			)
		};

		Iter {
			head,
			curr,
		}
	}

}

impl<'iter> Iterator for Iter<'iter> {
	type Item = &'iter FreeNode;
	fn next(&mut self) -> Option<Self::Item> {
		let head = self.head.as_ref()?;
		let curr = unsafe { self.curr.as_mut() };
		let next = unsafe { curr.next.as_mut() };

		if  next == *head  {
			self.head = None;
		}

		self.curr = curr.next;
		Some(curr)
	}
}

pub(super) mod test {
	use super::free_node::node_tests::new_node;
	use super::*;
	use kfs_macro::ktest;

	const PAGE_SIZE: usize = 4096;
	static mut PAGE1: [u8; PAGE_SIZE] = [0; PAGE_SIZE];

	#[ktest]
	fn test_new() {
		let mut list = FreeList::new();
		let page = unsafe { &mut PAGE1 };
		list.insert(new_node(page, 0, PAGE_SIZE));

		let head = list.head.unwrap().as_ptr().cast_const() as *const u8;
		assert_eq!(head, unsafe { PAGE1.as_ptr() })
	}

	#[ktest]
	fn test_last() {
		let mut list = FreeList::new();
		let page = unsafe { &mut PAGE1 };
		list.insert(new_node(page, 0, 32));

		let node = new_node(page, 50, 22);
		list.insert(node);

		let last = list.iter_mut().last().unwrap();
		assert_eq!(last.addr(), node.as_mut_ptr());
	}


	#[ktest]
	fn test_insert_merged() {

		fn init_list() -> FreeList {
			let page = unsafe { &mut PAGE1 };
			let mut list = FreeList::new();
			list.insert(new_node(page, 0, 30));
			list.insert(new_node(page, 30, 20));
			list
		}

		let list = init_list();
		let head = unsafe { list.head.unwrap().as_mut() };
		assert_eq!(head.bytes(), 50) // 30 + 20 = 50
	}

	#[ktest]
	fn test_insert() {

		fn init_list() -> FreeList {
			let page = unsafe { &mut PAGE1 };
			let mut list = FreeList::new();

			// insert tail
			list.insert(new_node(page, 100, 30));
			list.insert(new_node(page, 500, 20));
			list.insert(new_node(page, 1000, 100));
			list
		}

		fn index_of(list: &FreeList, node: &FreeNode) -> usize {
			let mut count = 0;
			for n in list.iter() {
				if n == node {
					break;
				}
				count += 1;
			}
			count
		}

		let page = unsafe { &mut PAGE1 };
		let mut list = init_list();

		// list:
		// addr 100 - 500 - 1000
		// size 30  - 20  - 100

		// insert head
		let node = new_node(page, 0, 31);
		list.insert(node);
		assert_eq!(index_of(&list, node), 0);

		// list:
		// addr 0  - 100 - 500 - 1000
		// size 31 - 30  - 20  - 100

		// insert mid
		let node = new_node(page, 50, 25);
		list.insert(node);
		assert_eq!(index_of(&list, node), 1);
	}

	#[ktest]
	fn test_remove() {
		fn init_list<'a>() -> (FreeList,(&'a mut FreeNode, &'a mut FreeNode, &'a mut FreeNode)) {
			let page = unsafe { &mut PAGE1 };
			let mut list = FreeList::new();

			let node0 = unsafe { new_node(page, 0, 30).as_non_null().as_mut() };
			let node1 = unsafe { new_node(page, 50, 20).as_non_null().as_mut() };
			let node2 = unsafe { new_node(page, 100, 100).as_non_null().as_mut() };
			
			list.insert(node0);
			list.insert(node1);
			list.insert(node2);
			(list, (node0, node1, node2))
		}

		// remove last
		let ( mut list, nodes) = init_list();
		list.remove(nodes.2);
		assert_eq!(list.count(), 2);
		assert_eq!(list.head.unwrap(), nodes.0.as_non_null());

		// remove second
		let ( mut list, nodes) = init_list();
		list.remove(nodes.1);
		assert_eq!(list.count(), 2);
		assert_eq!(list.head.unwrap(), nodes.0.as_non_null());

		// remove first
		let ( mut list, nodes) = init_list();
		list.remove(nodes.0);
		assert_eq!(list.count(), 2);
		assert_eq!(list.head.unwrap(), nodes.1.as_non_null());

		// remove last
		list.remove(nodes.2);
		assert_eq!(list.count(), 1);
		assert_eq!(list.head.unwrap(), nodes.1.as_non_null());

		// remove first
		list.remove(nodes.1);
		assert_eq!(list.head, None);
	}

	#[ktest]
	fn test_remove_if() {

		fn init_list<'a>() -> (FreeList,(&'a mut FreeNode, &'a mut FreeNode, &'a mut FreeNode)) {
			let page = unsafe { &mut PAGE1 };
			let mut list = FreeList::new();

			let node0 = unsafe { new_node(page, 0, 30).as_non_null().as_mut() };
			let node1 = unsafe { new_node(page, 50, 20).as_non_null().as_mut() };
			let node2 = unsafe { new_node(page, 100, 100).as_non_null().as_mut() };
			
			list.insert(node0);
			list.insert(node1);
			list.insert(node2);
			(list, (node0, node1, node2))
		}

		let (mut list, nodes) = init_list();

		list.remove_if(|n| n.bytes() > 25);
		assert_eq!(list.count(), 2);
		assert_eq!(list.head.unwrap(), nodes.1.as_non_null());
		assert_eq!(list.last().unwrap(), nodes.2.as_non_null());
	}

	#[ktest]
	fn test_partition() {

		fn init_list() -> FreeList {
			let page = unsafe { &mut PAGE1 };
			let mut list = FreeList::new();
			list.insert(new_node(page, 0, 30));
			list.insert(new_node(page, 50, 20));
			list.insert(new_node(page, 100, 100));
			list
		}

		fn do_test<F: FnMut(&&mut FreeNode)->bool>(condition: F, ans:(usize, usize)) {
			let mut list = init_list();
			let (x, y) = list.iter_mut()
				.partition::<FreeList, _>(condition);

			assert_eq!(x.count(), ans.0);
			assert_eq!(y.count(), ans.1);
		}

		do_test(|n| n.bytes() > 0, (3, 0));
		do_test(|n| n.bytes() > 30, (1, 2));
		do_test(|n| n.bytes() > 100, (0, 3));
	}
}
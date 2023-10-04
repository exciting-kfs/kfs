use alloc::sync::{Arc, Weak};

pub trait ListNode<T: Sync + Eq> {
	fn get_prev(&self) -> Weak<T>;
	fn get_next(&self) -> Weak<T>;

	fn set_prev(&self, prev: Weak<T>);
	fn set_next(&self, next: Weak<T>);
}

#[derive(Debug)]
pub struct List<T: ListNode<T> + Sync + Eq> {
	head: Option<Arc<T>>,
}

impl<T: ListNode<T> + Sync + Eq> List<T> {
	pub const fn new() -> Self {
		Self { head: None }
	}

	pub fn push_back(&mut self, node: Arc<T>) {
		if let Some(next) = self.head.as_mut() {
			// pr_debug!("list: push_back: node");

			let node_weak = Arc::downgrade(&node);
			let prev_weak = next.get_prev();
			let next_weak = Arc::downgrade(&next);

			let prev = Weak::upgrade(&prev_weak).unwrap();

			prev.set_next(node_weak.clone());
			next.set_prev(node_weak.clone());

			node.set_prev(prev_weak);
			node.set_next(next_weak);
		} else {
			// pr_debug!("list: push_back: head");
			self.head = Some(node);
		}
	}

	pub fn pop_front(&mut self) -> Option<Arc<T>> {
		let n = self.head.as_mut().map(|n| n.clone());

		if let Some(n) = n.as_ref() {
			self.remove(n.clone());
		}

		n
	}

	pub fn move_to_back(&mut self, node: Arc<T>) {
		if let Some(head) = self.head.as_ref() {
			let next = node.get_next().upgrade().unwrap();

			if head != &next {
				// pr_debug!("list: move_to_back");
				self.remove(node.clone());
				self.push_back(node)
			}
		}
	}

	pub fn remove(&mut self, node: Arc<T>) {
		if let Some(head) = self.head.as_mut() {
			// pr_debug!("list: remove");
			let prev_weak = node.get_prev();
			let next_weak = node.get_next();
			let node_weak = Arc::downgrade(&node);
			let prev = prev_weak.upgrade().unwrap();
			let next = next_weak.upgrade().unwrap();

			if node == *head {
				if node == next {
					self.head = None
				} else {
					self.head = Some(next.clone());
				}
			}

			prev.set_next(next_weak);
			next.set_prev(prev_weak);

			node.set_next(node_weak.clone());
			node.set_prev(node_weak);
		}
	}
}

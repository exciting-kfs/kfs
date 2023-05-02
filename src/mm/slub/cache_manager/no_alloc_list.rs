use core::ptr::NonNull;
use core::fmt::Debug;
use core::mem;

#[allow(unused)]
#[derive(Debug)]
pub struct Node<T> {
	prev: NonNull<Node<T>>,
	next: NonNull<Node<T>>,
	data: T,
}

impl<T> Node<T> {
        pub const NODE_SIZE: usize = mem::size_of::<Node<T>>();
        
        /// Construct a Node<T> for memory chunk
        /// 
        /// # Safety
        /// 
        /// * The size of memory chunk must be bigger than Node::NODE_SIZE
        pub unsafe fn construct_at(mem: &mut [u8], data: T) -> &mut Self {
                let ptr = mem.as_mut_ptr() as *mut Self;
                let next = NonNull::new_unchecked(&mut (*ptr));
                let prev = next.clone();
                (*ptr) = Node { prev, next, data };
                &mut (*ptr)
	}

        pub fn data(&self) -> &T {
                &self.data
        }

        pub fn data_mut(&mut self) -> &mut T {
                &mut self.data
        }

        #[inline(always)]
	pub fn as_non_null(&mut self) -> NonNull<Node<T>> {
		unsafe { NonNull::new_unchecked(self as *mut Node<T>) }
	}

	#[inline(always)]
	pub fn addr(&self) -> *const Node<T> {
		self as *const Node<T>
	}

	#[inline(always)]
        pub fn as_mut_ptr(&mut self) -> *mut Self {
                self as *mut Self
        }

        #[inline(always)]
        pub fn as_ptr(&self) -> *const Self {
                self as *const Self
        }

	pub fn disjoint(&mut self) {
                let prev = unsafe { self.prev.as_mut() };
                let next = unsafe { self.next.as_mut() };

                prev.next = self.next;
                next.prev = self.prev;

                self.next = self.as_non_null();
                self.prev = self.as_non_null();
        }
}

impl<T> PartialEq for Node<T> {
        fn eq(&self, other: &Self) -> bool {
                self.as_ptr() == other.as_ptr()
        }
}

pub struct NAList<T>{
	head: Option<NonNull<Node<T>>>,
}

impl<T> NAList<T> {
	pub const fn new() -> Self {
		NAList { head: None }
	}

        pub fn count(&self) -> usize {
                self.iter().count()
        }

        pub fn find_if<F>(&mut self, mut f: F) -> Option<NonNull<Node<T>>>
        where F: FnMut(&Node<T>) -> bool
        {
                self.iter_mut().find(|n| f(n)).map(|n| n.as_non_null())
        }

	pub fn insert_front(&mut self, node: &mut Node<T>) {
                if let None = self.head {
                        let node_ptr = node.as_non_null();
                        self.head = Some(node_ptr);
                        return;
                }

                self.insert(node);
                let node_ptr = node.as_non_null();
                self.head = Some(node_ptr);
	}

        pub fn insert_back(&mut self, node: &mut Node<T>) {
                if let None = self.head {
                        let node_ptr = node.as_non_null();
                        self.head = Some(node_ptr);
                        return;
                }

                self.insert(node);
	}


        fn insert(&mut self, node: &mut Node<T>) {
		let head = unsafe { self.head.unwrap().as_mut() };
                let prev = unsafe { head.prev.as_mut() };
                let node_ptr = node.as_non_null();

                prev.next = node_ptr;
                head.prev = node_ptr;

                node.next = head.as_non_null();
                node.prev = prev.as_non_null();
        }


        pub fn remove_if<'page, F>(&mut self, f: F) -> Option<&'page mut Node<T>>
        where F: FnMut(&Node<T>) -> bool
        {
                self.find_if(f).map(|mut node_ptr|{
                        let node = unsafe { node_ptr.as_mut() };
                        self.remove(node);
                        node
                })
        }

        fn remove(&mut self, node: &mut Node<T>) {
                self.head.map(|mut head_ptr| {
                        let head = unsafe { head_ptr.as_mut() };
                        if node == head {
                                self.head = Some(head.next);
                        }

                        if head_ptr == head.next {
                                self.head = None;
                        }

                        node.disjoint()
                });
        }


        pub fn iter_mut(&mut self) -> IterMut<'_, T> {
                IterMut::new(self)
        }

        pub fn iter(&self) -> Iter<'_, T> {
                Iter::new(self)
        }
}


impl<T> Debug for NAList<T>
where T: Debug
{
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_list().entries(self).finish()
        }
}

impl<'a, T> Extend<&'a mut Node<T>> for NAList<T> {
        fn extend<I: IntoIterator<Item = &'a mut Node<T>>>(&mut self, iter: I) {
                iter.into_iter().for_each(|n| {
                        n.disjoint();
                        self.insert(n);
                })
        }
}

impl<T> Default for NAList<T> {
        fn default() -> Self {
                NAList::new()
        }
}



/// Iterator - IterMut

impl<'iter, T> IntoIterator for &'iter mut NAList<T> {
        type Item = &'iter mut Node<T>;
        type IntoIter = IterMut<'iter, T>;
        fn into_iter(self) -> Self::IntoIter {
            self.iter_mut()
        }
}

#[derive(Debug)]
pub struct IterMut<'iter, T> {
        head: Option<&'iter mut Node<T>>,
        curr: NonNull<Node<T>>,
}

impl<'iter, T> IterMut<'iter, T> {
        fn new(cont: &mut NAList<T>) -> Self {
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

impl<'iter, T> Iterator for IterMut<'iter, T> {
        type Item = &'iter mut Node<T>;
        fn next(&mut self) -> Option<Self::Item> {
                let head = self.head.as_ref()?;
                let curr = unsafe { self.curr.as_mut() };
                let next = unsafe { curr.next.as_mut() };
                
                if next == *head || curr == next { // hmm
                        self.head = None;
                }

                self.curr = curr.next;
                Some(curr)
        }
}

/// Iterator - Iter

impl<'iter, T> IntoIterator for &'iter NAList<T> {
        type Item = &'iter T;
        type IntoIter = Iter<'iter, T>;
        fn into_iter(self) -> Self::IntoIter {
            self.iter()
        }
}

#[derive(Debug)]
pub struct Iter<'iter, T> {
        head: Option<&'iter Node<T>>,
        curr: NonNull<Node<T>>
}

impl<'iter, T> Iter<'iter, T> {
        fn new(cont: &NAList<T>) -> Self {
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

impl<'iter, T> Iterator for Iter<'iter, T> {
        type Item = &'iter T;
        fn next(&mut self) -> Option<Self::Item> {
                let head = self.head.as_ref()?;
                let curr = unsafe { self.curr.as_mut() };
                let next = unsafe { curr.next.as_mut() };

                if  next == *head  {
                        self.head = None;
                }

                self.curr = curr.next;
                Some(&curr.data)
        }
}
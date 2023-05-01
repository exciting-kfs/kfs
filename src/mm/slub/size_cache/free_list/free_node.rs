use core::ptr::NonNull;
use core::mem::size_of;

use crate::mm::slub::{PAGE_SIZE, PAGE_ALIGN};
use crate::mm::util::{is_aligned, prev_align, next_align};

use super::FreeList;

#[derive(Debug, PartialEq)]
pub struct TooBigError;

#[derive(Debug)]
pub struct FreeNode {
    pub prev: NonNull<FreeNode>,
    pub next: NonNull<FreeNode>,
    pub bytes: usize,
}

impl PartialEq for FreeNode {
        fn eq(&self, other: &Self) -> bool {
                self.as_ptr() == other.as_ptr()
        }
}

impl FreeNode {
        pub const NODE_SIZE: usize = size_of::<FreeNode>();

        /// Construct a FreeNode for memory chunk
        /// 
        /// # Safety
        /// 
        /// * The size of memory chunk must be bigger than FreeNode::NODE_SIZE
	pub unsafe fn construct_at(mem: &mut [u8]) -> &mut Self {
                let bytes = mem.len();
                let mem = mem.as_mut_ptr() as *mut Self;
                
                let next = NonNull::new_unchecked(&mut (*mem));
                let prev = next.clone();
                (*mem) = FreeNode { prev, next, bytes };
                &mut (*mem)
	}

        /// Try to merge `self` with previous and next node.
        /// If `self` doesn't have previous or next node, this function do nothing.
        pub fn try_merge(&mut self) {
                let (front, back) = unsafe {
                        (self.prev.as_mut(), self.next.as_mut())
                };

                let self_start = self.as_ptr().cast::<u8>();
                let back_start = back.as_ptr().cast::<u8>();
                let front_start = front.as_ptr().cast::<u8>();

                let self_end = unsafe { self_start.offset(self.bytes as isize) };
                let front_end = unsafe { front_start.offset(front.bytes as isize) };

                if self_end == back_start  {
                        let next = unsafe { back.next.as_mut() };
                        next.prev = self.as_non_null();
                        self.next = back.next;
                        self.bytes += back.bytes;
                }

                if front_end == self_start  {
                        let back = unsafe { self.next.as_mut() };
                        back.prev = self.prev;
                        front.next = self.next;
                        front.bytes += self.bytes;
                }
        }

        pub fn disjoint(&mut self) {
                let prev = unsafe { self.prev.as_mut() };
                let next = unsafe { self.next.as_mut() };

                prev.next = self.next;
                next.prev = self.prev;

                self.next = self.as_non_null();
                self.prev = self.as_non_null();
        }

        pub fn contains<T>(&self, ptr: *mut T) -> bool {
                let ptr = ptr.cast::<u8>().cast_const();
                let s = self.addr().cast::<u8>();
                let e = unsafe { s.offset(self.bytes() as isize) };
                e > ptr &&  ptr >= s
        }

        /// It is called by `CacheManager` for collecting excess cache memory allcated by `PAGE_ALLOC`.
        ///
        /// `'=': inuse, '-': free`
        /// 
        /// * case 0)
        /// ```
        /// align0     align1
        /// start       end
        ///   |----((----|
        ///   |----))----|
        ///
        /// ```
        /// * case 1)
        /// ```
        /// align0
        /// start    align1   end
        ///   |---((---|-------|
        ///   |---))---|-------|
        ///
        /// ```
        /// * case 2)
        /// ```
        ///                          align2
        /// align0  start   align1    end
        ///   |=======|-------|---((---|
        ///   |=======|-------|---))---|
        ///   
        /// ```
        /// * case 3)
        /// ```
        /// align0  start   align1   align2   end
        ///   |=======|-------|---((---|-------|
        ///   |=======|-------|---))---|-------|
        /// ```
        ///
        pub fn shrink(&mut self, free_list: &mut FreeList) -> (*mut u8, usize) {
                let mut total = self.bytes;
                let start = self.as_mut_ptr().cast::<u8>() as usize;
                let end = match start.checked_add(total) {
                        Some(e) => e,
                        None => 0,
                };
                let next_align = next_align(start, PAGE_ALIGN);

                if !is_aligned(end, PAGE_ALIGN) {
                        let len = end - prev_align(end, PAGE_ALIGN);
                        let n = unsafe {
                                FreeNode::construct_at(
                                        core::slice::from_raw_parts_mut(end as *mut u8, len)
                                )
                        };
                        total -= n.bytes;
                        free_list.insert(n);
                }

                if !is_aligned(start, PAGE_ALIGN) {
                        self.bytes = next_align - start;
                        total -= self.bytes;
                        free_list.insert(self);
                }

                (next_align as *mut u8 , total / PAGE_SIZE)
        }

        #[inline(always)]
        pub fn bytes(&self) -> usize {
                self.bytes
        }

        #[inline(always)]
        pub fn as_mut_ptr(&mut self) -> *mut Self {
                self as *mut Self
        }

        #[inline(always)]
        pub fn as_ptr(&self) -> *const Self {
                self as *const Self
        }

        #[inline(always)]
        pub fn as_non_null(&mut self) -> NonNull<Self> {
                unsafe { NonNull::new_unchecked(self.as_mut_ptr()) }
        }

        #[inline(always)]
        pub fn addr(&self) -> *const FreeNode {
                self as *const FreeNode
        }
}

pub(super) mod node_tests {

        use kfs_macro::ktest;

        use super::*;

        const PAGE_SIZE: usize = 4096;
        static mut PAGE1: [u8; PAGE_SIZE] = [0; PAGE_SIZE];

        pub fn new_node(page: &mut [u8], offset:usize, bytes: usize) -> &mut FreeNode {
                let ptr = unsafe { (page as *mut [u8] as *mut u8).offset(offset as isize) };
                let ptr = unsafe { core::slice::from_raw_parts_mut(ptr, bytes) };
                unsafe { FreeNode::construct_at(ptr) }
        }

        #[ktest]
        fn test_construct_at() {
                let page = unsafe { &mut PAGE1 };
                let page_ptr = page.as_mut_ptr() as *mut FreeNode;
                let node = unsafe { FreeNode::construct_at(page) };

                assert_eq!(node.next.as_ptr(), page_ptr);
                assert_eq!(node.prev.as_ptr(), page_ptr);
                assert_eq!(node.as_ptr(), page_ptr);
        }

        #[ktest]
        fn test_try_merge() {

                fn init_nodes<'a>() -> (&'a mut FreeNode, &'a mut FreeNode) {
                        let page = unsafe { &mut PAGE1 };

                        let node1 = new_node(page, 30, 30);
                        let mut node1_ptr = node1.as_non_null();
                        let node1 = unsafe { node1_ptr.as_mut() };

                        let node0 = new_node(page, 0, 30);
                        let node0_ptr = node0.as_non_null();

                        node1.prev = node0_ptr;
                        node1.next = node0_ptr;
                        node0.prev = node1_ptr;
                        node0.next = node1_ptr;
                        (node0, node1)
                }
                
                // merge with next node.
                let (node0, _) = init_nodes();
                let node0_ptr = node0.as_non_null();
                node0.try_merge();
                assert_eq!(node0.bytes, 60);
                assert_eq!(node0.prev, node0_ptr);
                assert_eq!(node0.next, node0_ptr);

                // merge with previous node.
                let (node0, node1) = init_nodes();
                let node0_ptr = node0.as_non_null();
                node1.try_merge();
                assert_eq!(node0.bytes, 60);
                assert_eq!(node0.prev, node0_ptr);
                assert_eq!(node0.next, node0_ptr);

                // Never merge.
                let node2 = new_node(unsafe { &mut PAGE1 }, 300, 30);
                let node2_ptr = node2.as_non_null();
                node2.try_merge();
                assert_eq!(node0.bytes, 60);
                assert_eq!(node0.prev, node0_ptr);
                assert_eq!(node0.next, node0_ptr);
                assert_eq!(node2.bytes, 30);
                assert_eq!(node2.prev, node2_ptr);
                assert_eq!(node2.next, node2_ptr);
        }
        
        #[ktest]
        fn test_disjoint() {

                fn do_test(node: &mut FreeNode, left: &FreeNode, right: &FreeNode) {
                        let node_ptr = node.as_non_null();
                        node.disjoint();
                        assert_eq!(node.prev, node_ptr);
                        assert_eq!(node.next, node_ptr);
        
                        assert_ne!(left.next, node_ptr);
                        assert_ne!(right.prev, node_ptr);
                }
        
                fn make_3nodes_jointed<'a>() -> (&'a mut FreeNode, &'a mut FreeNode, &'a mut FreeNode) {
                        let page = unsafe { &mut PAGE1 };
        
                        let node0 = new_node(page, 0, 31);
                        let mut node0_ptr = node0.as_non_null();
                        let node0 = unsafe { node0_ptr.as_mut() };
        
                        let node1 = new_node(page, 100, 32);
                        let mut node1_ptr = node1.as_non_null();
                        let node1 = unsafe { node1_ptr.as_mut() };
        
                        let node2 = new_node(page, 200, 33);
                        let mut node2_ptr = node2.as_non_null();
                        let node2 = unsafe { node2_ptr.as_mut() };
        
                        node0.prev = node2_ptr;
                        node0.next = node1_ptr;
        
                        node1.prev = node0_ptr;
                        node1.next = node2_ptr;
        
                        node2.prev = node1_ptr;
                        node2.next = node0_ptr;
        
                        (node0, node1, node2)
                }
        

                // disjoint first
                let nodes = make_3nodes_jointed();
                do_test(nodes.0, nodes.2, nodes.1);

                // disjoint second
                let nodes = make_3nodes_jointed();
                do_test(nodes.1, nodes.0, nodes.2);

                // disjoint last
                let nodes = make_3nodes_jointed();
                do_test(nodes.2, nodes.1, nodes.0);
        }
}


use core::ptr::NonNull;
use core::mem::size_of;
use core::slice;

use crate::mm::cache_sw::PAGE_SIZE;

#[derive(Debug)]
pub struct FreeNode {
    pub prev: NonNull<FreeNode>,
    pub next: NonNull<FreeNode>,
    bytes: usize,
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

        pub fn from_non_null<'a>(mut ptr: NonNull<Self>) -> &'a mut Self { // safety ?
                unsafe { ptr.as_mut() }
        }

        /// Try to merge self with previous and next node.
        /// If self doesn't have previous or next node, this function do nothing.
        pub fn try_merge(&mut self) {
                let front = unsafe { self.prev.as_mut() };
                let back = unsafe { self.next.as_mut() };

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

        /// Allocate a memory block of requested bytes from Self.
        /// After allocation, if there is a remained block, make it to FreeNode and return it.
        /// 
        /// # Safety
        /// 
        /// You have to concern for size of remains. It must be bigger than FreeNode::NODE_SIZE.
        pub unsafe fn alloc_bytes(&mut self, bytes: usize) -> Option<&mut Self> {
                self.bytes.checked_sub(bytes).filter(|x| *x != 0).map(|remains| {
                        let ptr = self.as_mut_ptr().cast::<u8>().offset(bytes as isize);
                        let ptr = slice::from_raw_parts_mut(ptr, remains);
                        let new_node = FreeNode::construct_at(ptr);
                        new_node
                })
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

        pub fn disjoint(&mut self) {
                let prev = Self::from_non_null(self.prev);
                let next = Self::from_non_null(self.next);

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

        /// case 1)
        /// align
        ///  ptr
        ///  self     node_ptr
        ///   |----((----|---------|
        ///   |----))----|---------|
        ///              `node_size`
        /// case 2)
        /// align        self      ptr      node_ptr
        ///   |===========|---------|----((----|---------|
        ///   |===========|---------|----))----|---------|
        ///   `self_offset`self_size`          `node_size`
        pub fn shrink(&mut self) -> (*mut u8, usize, Option<&mut Self>) {

                let self_ptr = self.as_ptr() as usize;
                let self_offset = self_ptr % PAGE_SIZE;
                let is_remain = (self_offset > 0) as usize;
                let self_size = is_remain * (PAGE_SIZE - self_offset);

                let total = self.bytes;
                self.bytes = self_size;

                let ptr = self_ptr + self_size;
                let count = (total - self_size) / PAGE_SIZE;

                let node_ptr = ptr + PAGE_SIZE * count;
                let node_size = total - (node_ptr - self_ptr);
                let new_node = (node_size > 0).then(|| unsafe {
                        let node_ptr = node_ptr as *mut u8;
                        let node_ptr = slice::from_raw_parts_mut(node_ptr, node_size);
                        FreeNode::construct_at(node_ptr)
                });

                (ptr as *mut u8, count, new_node)
        }
}

pub(super) mod node_tests {

        use kfs_macro::kernel_test;

        use super::FreeNode;

        const PAGE_SIZE: usize = 4096;
        static mut PAGE1: [u8; PAGE_SIZE] = [0; PAGE_SIZE];

        pub fn new_node(page: &mut [u8], offset:usize, bytes: usize) -> &mut FreeNode {
                let ptr = unsafe { (page as *mut [u8] as *mut u8).offset(offset as isize) };
                let ptr = unsafe { core::slice::from_raw_parts_mut(ptr, bytes) };
                unsafe { FreeNode::construct_at(ptr) }
        }

        #[kernel_test(cache_free_node)]
        fn test_inject() {
                let page = unsafe { &mut PAGE1 };
                let page_ptr = page.as_mut_ptr() as *mut FreeNode;
                let node = unsafe { FreeNode::construct_at(page) };

                assert_eq!(node.next.as_ptr(), page_ptr);
                assert_eq!(node.prev.as_ptr(), page_ptr);
                assert_eq!(node.as_ptr(), page_ptr);
        }

        fn try_merge_init_nodes<'a>() -> (&'a mut FreeNode, &'a mut FreeNode) {
                let page = unsafe { &mut PAGE1 };

                let node1 = new_node(page, 30, 30);
                let node1_ptr = node1.as_non_null();
                let node1 = FreeNode::from_non_null(node1_ptr);

                let node0 = new_node(page, 0, 30);
                let node0_ptr = node0.as_non_null();

                node1.prev = node0_ptr;
                node1.next = node0_ptr;
                node0.prev = node1_ptr;
                node0.next = node1_ptr;
                (node0, node1)
        }

        #[kernel_test(cache_free_node)]
        fn test_try_merge() {
                

                let (node0, _) = try_merge_init_nodes();
             
                let node0_ptr = node0.as_non_null();
                node0.try_merge();
                assert_eq!(node0.bytes, 60);
                assert_eq!(node0.prev, node0_ptr);
                assert_eq!(node0.next, node0_ptr);

                let (node0, node1) = try_merge_init_nodes();
             
                let node0_ptr = node0.as_non_null();
                node1.try_merge();
                assert_eq!(node0.bytes, 60);
                assert_eq!(node0.prev, node0_ptr);
                assert_eq!(node0.next, node0_ptr);
        }

        #[kernel_test(cache_free_node)]
        fn test_alloc_bytes() {
                let page = unsafe { &mut PAGE1 };
                let node = unsafe { FreeNode::construct_at(page) };

                let bytes = 10000;
                let res1 = unsafe { node.alloc_bytes(bytes) };
                if res1.is_some() {
                        assert!(false);
                }

                let bytes = 10;
                let res2 = unsafe { node.alloc_bytes(bytes) };
                let next = res2.unwrap();

                let ptr2 = next.addr().cast::<u8>();
                let ptr1 = unsafe { node.addr().cast::<u8>().offset(bytes as isize) };

                assert_eq!(ptr1, ptr2);
        }

        fn disjoint_do_test(node: &mut FreeNode, left: &FreeNode, right: &FreeNode) {
                let node_ptr = node.as_non_null();
                node.disjoint();
                assert_eq!(node.prev, node_ptr);
                assert_eq!(node.next, node_ptr);

                assert_ne!(left.next, node_ptr);
                assert_ne!(right.prev, node_ptr);
        }

        fn disjoint_init_nodes<'a>() -> (&'a mut FreeNode, &'a mut FreeNode, &'a mut FreeNode) {
                let page = unsafe { &mut PAGE1 };

                let node0 = new_node(page, 0, 31);
                let node0_ptr = node0.as_non_null();
                let node0 = FreeNode::from_non_null(node0_ptr);

                let node1 = new_node(page, 100, 32);
                let node1_ptr = node1.as_non_null();
                let node1 = FreeNode::from_non_null(node1_ptr);

                let node2 = new_node(page, 200, 33);
                let node2_ptr = node2.as_non_null();
                let node2 = FreeNode::from_non_null(node2_ptr);

                node0.prev = node2_ptr;
                node0.next = node1_ptr;

                node1.prev = node0_ptr;
                node1.next = node2_ptr;

                node2.prev = node1_ptr;
                node2.next = node0_ptr;

                (node0, node1, node2)
        }

        #[kernel_test(cache_free_node)]
        fn test_disjoint() {

                // first
                let nodes = disjoint_init_nodes();
                disjoint_do_test(nodes.0, nodes.2, nodes.1);

                // second
                let nodes = disjoint_init_nodes();
                disjoint_do_test(nodes.1, nodes.0, nodes.2);

                // last
                let nodes = disjoint_init_nodes();
                disjoint_do_test(nodes.2, nodes.1, nodes.0);
        }
}


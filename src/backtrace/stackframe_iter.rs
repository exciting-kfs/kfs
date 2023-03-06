use super::stackframe::{
    self, Stackframe
};

extern "C" {
    /// the start address of the stack.
    fn stack_init();
}

pub struct StackframeIter {
    pub(super) base_ptr: *const usize
}

impl Iterator for StackframeIter {
    type Item = Stackframe;
    fn next(&mut self) -> Option<Self::Item> {
        let stack_base = stack_init as *const usize;
        if self.base_ptr == stack_base {
            return None
        }

        let ret = Some(Stackframe::new(self.base_ptr));
        self.base_ptr = stackframe::next(self.base_ptr);
        ret
    }
}
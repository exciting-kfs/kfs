use super::wrap_queue::WrapQueue;

pub struct SizedPool<T, const CAP: usize> {
        pool: [T; CAP],
        recycle: WrapQueue<usize, CAP>
}

impl<T, const CAP: usize> SizedPool<T, CAP> {
        pub fn from_fn<F>(cb: F) -> Self
        where F: FnMut(usize) -> T
        {
                SizedPool { pool: core::array::from_fn(cb), recycle: WrapQueue::filled(|idx| idx) }
        }

        pub fn insert(&mut self, data: T) -> Option<usize>{
                self.recycle.pop().and_then(|idx| {
                        self.pool[idx] = data;
                        Some(idx)
                })
        }

        pub fn remove(&mut self, idx: usize) {
                self.recycle.push(idx);
        }

        pub fn take(&mut self, idx: usize) -> Option<T>
        where T: Default
        {
                match self.is_avail(idx) {
                        true => {
                                self.recycle.push(idx);
                                Some(core::mem::take(&mut self.pool[idx]))
                        }
                        false => None,
                }
        }

        fn is_avail(&self, idx: usize) -> bool {
                let recycle = &self.recycle;

                for r in recycle {
                        if *r == idx {
                                return false
                        }
                }
                true
        }

        pub fn at(&self, idx: usize) -> Option<&T> {
                match self.is_avail(idx) {
                        true => Some(&self.pool[idx]),
                        false => None
                }
        }

        pub fn at_mut(&mut self, idx: usize) -> Option<&mut T> {
                match self.is_avail(idx) {
                        true => Some(&mut self.pool[idx]),
                        false => None
                }
        }

        pub fn iter_mut(&mut self) -> IterMut<'_, T, CAP> {
                IterMut::new(self)
        }

        pub fn iter(&self) -> Iter<'_, T, CAP> {
                Iter::new(self)
        }
}

impl<'a, T, const CAP:usize> IntoIterator for &'a mut SizedPool<T, CAP> {
        type Item = &'a mut T;
        type IntoIter = IterMut<'a, T, CAP>;
        fn into_iter(self) -> Self::IntoIter {
            self.iter_mut()
        }
}

impl<'a, T, const CAP:usize> IntoIterator for &'a SizedPool<T, CAP> {
        type Item = &'a T;
        type IntoIter = Iter<'a, T, CAP>;
        fn into_iter(self) -> Self::IntoIter {
            self.iter()
        }
}


/// IterMut

pub struct IterMut<'a, T, const CAP: usize> {
        idx: usize,
        slice: &'a mut [T]
}

impl<'a, T, const CAP: usize> IterMut<'a, T, CAP> {
        fn new(container: &'a mut SizedPool<T, CAP>) -> Self {
                IterMut { idx: 0, slice: &mut container.pool, }
        }
}

/// fn next에서의 reference를 <'b>라고 하자. return 값은 <'a>의 라이프타임을 가져야 하므로,
/// 라이프타임 스코프가 작은 <'b>에서 벗어나기 위해 self.slice에 take를 하여 <'a>의 라이프타임 스코프를 가지는
/// 임시 레퍼런스 tmp를 만든다.
impl<'a, T, const CAP: usize> Iterator for IterMut<'a, T, CAP>
{
        type Item = &'a mut T;
        fn next(&mut self) -> Option<Self::Item> {
		let idx = self.idx;
                let slice = &mut self.slice;

                (idx < CAP).then_some({
                        let tmp = core::mem::take(slice);
                        let (head, tail) = tmp.split_at_mut(1);
        
                        self.slice = tail;
                        self.idx += 1;
                        &mut head[0]
                })
        }
}

/// Iter
pub struct Iter<'a, T, const CAP: usize> {
        idx: usize,
        slice: &'a [T]
}

impl<'a, T, const CAP: usize> Iter<'a, T, CAP> {
        fn new(container: &'a SizedPool<T, CAP>) -> Self {
                Iter { idx: 0, slice: &container.pool, }
        }
}

impl<'a, T, const CAP: usize> Iterator for Iter<'a, T, CAP>
{
        type Item = &'a T;
        fn next(&mut self) -> Option<Self::Item> {
		let idx = self.idx;

                (idx < CAP).then_some({
                        self.idx += 1;
                        &self.slice[idx]
                })
        }
}


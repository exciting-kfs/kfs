use core::ops::{Index, IndexMut};

#[derive(PartialEq)]
enum State {
    Empty,
    Full,
    Avail,
}

pub struct WrapQueue<T, const N: usize> {
    data: [T; N],
    head: usize,
    tail: usize,
    state: State,
}

impl<T, const CAPACITY: usize> WrapQueue<T, CAPACITY> {
    pub fn from_fn<F>(cb: F) -> Self
    where
        F: FnMut(usize) -> T,
    {
        Self {
            data: core::array::from_fn(cb),
            head: 0,
            tail: 0,
            state: State::Empty,
        }
    }

    fn translate_idx(&self, idx: usize) -> Option<usize> {
        if idx >= self.size() {
            None
        } else {
            Some((self.head + idx) % CAPACITY)
        }
    }

    pub fn size(&self) -> usize {
        match self.state {
            State::Full => CAPACITY,
            State::Empty => 0,
            State::Avail => {
                if self.head < self.tail {
                    self.tail - self.head
                } else {
                    self.tail + CAPACITY - self.head - 1
                }
            }
        }
    }

    fn circular_next(n: usize) -> usize {
        (n + 1) % CAPACITY
    }

    fn circular_prev(n: usize) -> usize {
        if n == 0 {
            CAPACITY - 1
        } else {
            n - 1
        }
    }

    pub fn empty(&self) -> bool {
        self.state == State::Empty
    }

    pub fn full(&self) -> bool {
        self.state == State::Full
    }

    pub fn at_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.translate_idx(idx).map(|i| &mut self.data[i])
    }

    pub fn at(&self, idx: usize) -> Option<&T> {
        self.translate_idx(idx).map(|i| &self.data[i])
    }

    pub fn extend(&mut self, n: usize) {
        for _ in 0..n {
            if self.full() {
                self.head = Self::circular_next(self.head);
            }

            self.tail = Self::circular_next(self.tail);

            self.state = match self.tail == self.head {
                true => State::Full,
                false => State::Avail,
            };
        }
    }

    pub fn push(&mut self, item: T) {
        self.data[self.tail] = item;
        self.extend(1)
    }

    pub fn window<'a>(&'a self, start: usize, size: usize) -> Option<Window<&'a [T], CAPACITY>> {
        if size == 0 {
            return None;
        }

        let head = self.translate_idx(start)?;
        let tail = self.translate_idx(start + size - 1)? + 1;

        Some(Window { head, tail, data: &self.data })
    }

    pub fn window_mut<'a>(&'a mut self, start: usize, size: usize) -> Option<Window<&'a mut [T], CAPACITY>> {
        if size == 0 {
            return None;
        }

        let head = self.translate_idx(start)?;
        let tail = self.translate_idx(start + size - 1)? + 1;

        Some(Window { head, tail, data: &mut self.data })
    }
}

pub struct Window<T, const N: usize> {
    head: usize,
    tail: usize,
    data: T,
}

impl<'a, T, const CAPACITY: usize> Index<usize> for Window<&'a [T], CAPACITY> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let index = (self.head + index ) % CAPACITY;
        
        &self.data[index]
    }
}

impl<'a, T, const CAPACITY: usize> Index<usize> for Window<&'a mut [T], CAPACITY> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let index = (self.head + index ) % CAPACITY;
        
        &self.data[index]
    }
}

impl<'a, T, const CAPACITY: usize> IndexMut<usize> for Window<&'a mut [T], CAPACITY> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let index = (self.head + index ) % CAPACITY;

        &mut self.data[index]
    }
}

impl<'a, T, const N: usize> IntoIterator for Window<&'a [T], N> {
    type Item = &'a [T];
    type IntoIter = core::array::IntoIter<Self::Item, 2>;

    fn into_iter(self) -> Self::IntoIter {
        match self.head < self.tail {
            true => [&self.data[self.head..self.tail], &[]],
            false => [&self.data[self.head..], &self.data[0..self.tail]]
        }.into_iter()
    }
}
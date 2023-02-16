use core::ops::Index;

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

    pub fn view<'a>(&'a self, start: usize, size: usize) -> Option<Window<'a, T>> {
        if size == 0 {
            return None;
        }

        // i hope...
        let head = self.translate_idx(start)?;
        let tail = self.translate_idx(start + size - 1)? + 1;

        if head < tail {
            Some(Window {
                parts: [&self.data[head..tail], &[]],
            })
        } else {
            Some(Window {
                parts: [&self.data[head..], &self.data[0..tail]],
            })
        }
    }
}

pub struct Window<'a, T> {
    parts: [&'a [T]; 2],
}

impl<'a, T> Index<usize> for Window<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let low_size = self.parts[0].len();

        if low_size > index {
            &self.parts[0][index]
        } else {
            &self.parts[1][index - low_size]
        }
    }
}

impl<'a, T> IntoIterator for Window<'a, T> {
    type Item = &'a [T];
    type IntoIter = core::array::IntoIter<Self::Item, 2>;

    fn into_iter(self) -> Self::IntoIter {
        self.parts.into_iter()
    }
}
use core::marker::PhantomData;

/// Symbol table in the '.symtab' section.
pub struct Symtab {
    addr: *const SymtabEntry,
    count: usize
}

impl Symtab {
    pub const fn new() -> Self {
        Symtab { addr: 0 as *const SymtabEntry, count: 0 }
    }

    pub fn init(&mut self, addr: *const SymtabEntry, count: usize) {
        *self = Symtab { addr, count }
    }

    /// Find the symtab entry and return index of name used in the strtab.
    pub fn get_name_index(&self, fn_addr: *const usize) -> Option<isize> {
        self.iter()
            .find(|entry| entry.st_value as *const usize == fn_addr)
            .map(|entry| entry.st_name as isize)
    }

    pub fn get_addr(&self, index: usize) -> Option<*const usize> {
        self.iter()
            .find(|entry| entry.st_name as usize == index)
            .map(|entry| entry.st_value as *const usize)
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self)
    }
}

impl<'a> IntoIterator for &'a Symtab {
    type Item = &'a SymtabEntry;
    type IntoIter = Iter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct Iter<'a> {
    ptr: *const SymtabEntry,
    len: usize,
    count: usize,
    p: PhantomData<&'a u8>
}

impl<'a> Iter<'a> {
    fn new(cont: &Symtab) -> Self {
        Iter { ptr: cont.addr, len: cont.count, count: 0, p: PhantomData }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a SymtabEntry;
    fn next(&mut self) -> Option<Self::Item> {
        if self.len == self.count {
            return None;
        }

        let entry = unsafe { &*self.ptr.add(self.count) };
        self.count += 1;
        Some(entry)
    }
}

/// Symbol table entry
pub struct SymtabEntry {
    st_name: u32,
    st_value: u32,
    st_size: u32,
    st_info: u8,
    st_other: u8,
    st_shndx: u16,
}

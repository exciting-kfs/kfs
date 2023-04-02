/// Symbol table in the '.symtab' section.
pub struct Symtab {
    addr: *const SymtabEntry,
    count: usize
}

impl Symtab {
    pub const fn new() -> Self {
        Symtab { addr: 0 as *const SymtabEntry, count: 0 }
    }

    pub fn init(addr: *const SymtabEntry, count: usize) -> Self {
        Symtab { addr, count }
    }

    /// Find the symtab entry and return index of name used in the strtab.
    pub fn get_name_index(&self, fn_addr: *const usize) -> Option<isize> {
        let mut ret = None;
        unsafe {
            for c in 0..self.count {
                let entry = &*self.addr.offset(c as isize);
                if entry.st_value == fn_addr as u32 {
                    ret = Some(entry.st_name as isize)
                }
            }
        }
        ret
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

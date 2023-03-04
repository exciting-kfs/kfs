use core::ffi::{CStr, c_char};

pub struct Strtab {
    addr: *const u8
}

impl Strtab {
    pub fn new(addr: *const u8) -> Self {
        Strtab { addr }
    }
    
    pub fn get_name(&self, index: Option<isize>) -> Option<&'static str> {
        index.map(|idx| {
            let start = unsafe { self.addr.offset(idx) } as *const c_char;
            unsafe { CStr::from_ptr(start).to_str().expect("invalid strtab or index.") }
        })
    }
}

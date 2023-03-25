use bitflags::bitflags;

bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PageFlag: u32 {
        const Present = 1;
        const Write = 2;
        const User = 4;
        const PWT = 8;
        const PCD = 16;
        const Accessed = 32;
        const Dirty = 64;
        const Global = 256;
    }
}

#[repr(transparent)]
pub struct PDE {
    data: PageFlag,
}

impl PDE {
    const PSE: u32 = 128;
    const ADDR_MASK_4M: u32 = 0b11111111_11000000_00000000_00000000;
    const ADDR_MASK: u32 = 0b11111111_11111111_11110000_00000000;

    pub fn new_4m(addr: usize, flags: PageFlag) -> Self {
        Self {
            data: PageFlag::from_bits_retain((addr as u32 & Self::ADDR_MASK_4M) | Self::PSE)
                | flags,
        }
    }

    pub fn new(addr: usize, flags: PageFlag) -> Self {
        Self {
            data: PageFlag::from_bits_retain(addr as u32 & Self::ADDR_MASK) | flags,
        }
    }
}

impl AsMut<PageFlag> for PDE {
    fn as_mut(&mut self) -> &mut PageFlag {
        &mut self.data
    }
}

#[repr(transparent)]
pub struct PTE {
    data: PageFlag,
}

impl PTE {
    const ADDR_MASK: u32 = 0b11111111_11111111_11110000_00000000;

    pub fn new(addr: usize, flags: PageFlag) -> Self {
        Self {
            data: PageFlag::from_bits_retain(addr as u32 & Self::ADDR_MASK) | flags,
        }
    }
}

impl AsMut<PageFlag> for PTE {
    fn as_mut(&mut self) -> &mut PageFlag {
        &mut self.data
    }
}

#[repr(C, align(4096))]
pub struct PT {
    pub entries: [PTE; 1024],
}

#[repr(C, align(4096))]
pub struct PD {
    pub entries: [PDE; 1024],
}

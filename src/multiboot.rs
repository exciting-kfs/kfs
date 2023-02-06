use core::mem::size_of;

#[repr(packed)]
#[allow(dead_code)]
struct Multiboot2EndTag {
    tag_type: u16,
    tag_flags: u16,
    tag_size: u32,
}

impl Multiboot2EndTag {
    const fn new() -> Self {
        Multiboot2EndTag {
            tag_type: 0,
            tag_flags: 0,
            tag_size: size_of::<Self>() as u32,
        }
    }
}

#[repr(packed)]
#[allow(dead_code)]
pub struct Multiboot2 {
    magic: u32,
    arch: u32,
    length: u32,
    checksum: u32,
    end_tag: Multiboot2EndTag,
}

impl Multiboot2 {
    // multiboot2 magic
    const MAGIC: u32 = 0xE85250D6;

    // 0 means x86.
    const ARCH: u32 = 0x0;

    const fn checksum() -> u32 {
        u32::wrapping_sub(0, Self::ARCH + Self::MAGIC + size_of::<Self>() as u32)
    }

    pub const fn new() -> Self {
        Multiboot2 {
            magic: Self::MAGIC,
            arch: Self::ARCH,
            length: size_of::<Self>() as u32,
            checksum: Self::checksum(),
            end_tag: Multiboot2EndTag::new(),
        }
    }
}

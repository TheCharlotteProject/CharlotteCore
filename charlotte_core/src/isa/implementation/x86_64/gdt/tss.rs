use core::mem::size_of;

#[repr(C, packed(1))]
pub struct Tss {
    res: u32,
    rsp0: u64,
    unused: [u8; 90],
    iopb: u16,
}

impl Tss {
    pub fn new(rsp0: u64) -> Self {
        Tss {
            res: 0,
            rsp0,
            unused: [0u8; 90],
            iopb: size_of::<Tss>() as u16,
        }
    }
}

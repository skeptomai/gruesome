mod object;

// NOTE: this is only up to v3
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Zobject {
    pub attribute_bits: [u8; 4],
    pub parent: u8,
    pub next: u8,
    pub child: u8,
    pub property_offset: [u8; 2],
}

impl Zobject {
    pub fn new(bytes: &[u8]) -> Zobject {
        let sz = std::mem::size_of::<Zobject>();
        let (_prefix, zobj, _suffix) = unsafe { &bytes[0..sz].align_to::<Zobject>() };
        zobj[0].clone()
    }

    pub fn attributes(&self) -> Vec<u8> {
        let mut attrs = vec![];
        let mut index = 0;
        for i in self.attribute_bits {
            let mut mask = 0x80;

            for _j in 0..8 {
                let r = mask & i;
                if r != 0 {
                    attrs.push(index);
                }
                mask >>= 1;
                index += 1;
            }
        }
        attrs
    }

    pub fn properties_addr(&self) -> u16 {
        u16::from_be_bytes(self.property_offset)
    }
}

impl Display for Zobject {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "Attributes: {:?}, Parent: {}, Next: {}, Child: {}, Properties Address {:#04x}",
            self.attributes(),
            self.parent,
            self.next,
            self.child,
            self.properties_addr()
        )
    }
}

pub struct ZobjectPostV3 {
    pub attributes: [u16; 3],
    pub parent: u16,
    pub next: u16,
    pub child: u16,
    pub property_offset: u16,
}

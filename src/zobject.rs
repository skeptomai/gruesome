use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

// In Versions 1 to 3, there are at most 255 objects, each having a 9-byte entry as follows
#[derive(Debug)]
pub struct ObjectTree {}

#[derive(Debug)]
pub struct ObjectTable<'a> {
    obj_raw: &'a [u8],
    pub objects: Vec<Zobject>,
}

impl<'a> ObjectTable<'a> {
    pub fn new(obj_table_addr: &'a [u8], num_obj: u16) -> Self {
        let mut base = 0;
        let mut n = num_obj;
        let mut objs = vec![];

        while n > 0 {
            let zobj = Zobject::new(&obj_table_addr[base..base + std::mem::size_of::<Zobject>()]);
            objs.push(zobj);
            n -= 1;
            base += std::mem::size_of::<Zobject>();
        }

        ObjectTable {
            obj_raw: obj_table_addr,
            objects: objs,
        }
    }
}

impl<'a> Display for ObjectTable<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        writeln!(f, "There are {} objects.", self.objects.len())?;
        for (i, x) in self.objects.iter().enumerate() {
            writeln!(
                f,
                "
            {}:
            {}",
                i + 1,
                x
            )?;
        }
        Ok(())
    }
}

// NOTE: this is only up to v3
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Zobject {
    pub attribute_bits: [u8; 4],
    pub parent: u8,
    pub next: u8,
    pub child: u8,
    pub properties_offsets: [u8; 2],
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
        u16::from_be_bytes(self.properties_offsets)
    }
}

impl Display for Zobject {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
                    f,
                    "
                    Attributes: {:?}, 
                    Parent: {}, Next: {}, Child: {}, 
                    Properties Address {:#04x},
                    Properties: {:?}
                    ",
                    self.attributes(),
                    self.parent,
                    self.next,
                    self.child,
                    self.properties_addr(),
                    self.properties_offsets,
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

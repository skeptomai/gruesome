use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

use crate::game::GameFile;
use crate::util::read_text;

// In Versions 1 to 3, there are at most 255 objects, each having a 9-byte entry as follows
#[derive(Debug)]
pub struct ObjectTree {}

#[derive(Debug)]
pub struct ObjectTable<'a> {
    obj_raw: &'a [u8],
    pub objects: Vec<Zobject>,
}

impl<'a> ObjectTable<'a> {
    pub fn new(gfile: &GameFile, obj_table_addr: &'a [u8], num_obj: u16) -> Self {
        let mut base = 0;
        let mut n = num_obj;
        let mut objs = vec![];

        while n > 0 {
            let zobj = Zobject::new(gfile, &obj_table_addr[base..base + std::mem::size_of::<InnerZobject>()]);
            objs.push(zobj);
            n -= 1;
            base += std::mem::size_of::<InnerZobject>();
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
pub struct InnerZobject {
    pub attribute_bits: [u8; 4],
    pub parent: u8,
    pub next: u8,
    pub child: u8,
    pub properties_offsets: [u8; 2],
}

#[derive(Debug)]
pub struct Zobject {
    zobj : InnerZobject,
    description : String,
}

impl Zobject {
    pub fn new(gfile: &GameFile, bytes: &[u8]) -> Zobject {
        let sz = std::mem::size_of::<InnerZobject>();
        let (_prefix, zobj, _suffix) = unsafe { &bytes[0..sz].align_to::<InnerZobject>() };

        let properties_addr = u16::from_be_bytes(zobj[0].properties_offsets) as usize;
        let description = if gfile.bytes[properties_addr] == 0 {"".to_string()}
        else {read_text(&gfile.bytes, properties_addr + 1, 
            gfile.memory_map.abbrev_strings as usize,
            gfile.memory_map.abbrev_table as usize).unwrap()};

        Zobject{ zobj: zobj[0], description: description}
    }

    pub fn attributes(&self) -> Vec<u8> {
        let mut attrs = vec![];
        let mut index = 0;
        for i in self.zobj.attribute_bits {
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
        u16::from_be_bytes(self.zobj.properties_offsets)
    }
}

impl Display for Zobject {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
                    f,
                    "
                    Attributes: {:?}, 
                    Parent object: {}, Sibling object: {}, Child object: {}, 
                    Property Address {:#04x},
                    Description: '{}',
                    Properties:
                    ",
                    self.attributes(),
                    self.zobj.parent,
                    self.zobj.next,
                    self.zobj.child,
                    self.properties_addr(),
                    self.description
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

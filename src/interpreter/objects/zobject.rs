use log::{debug, info};
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

// use crate::interpreter::text::dictionary::Dictionary;
use crate::interpreter::core::game::GameFile;

// 12.3.1
// In Versions 1 to 3, there are at most 255 objects, each having a 9-byte entry as follows:
//   the 32 attribute flags     parent     sibling     child   properties
//   ---32 bits in 4 bytes---   ---3 bytes------------------  ---2 bytes--
#[derive(Debug, Clone, Copy)]
pub struct ObjectTree {}

#[derive(Debug)]
pub struct ObjectTable {
    objects: Vec<Zobject>,
}

impl ObjectTable {
    /// Create an object table from gamefile
    /// which requires access to abbrevs, etc
    pub fn new(gfile: &GameFile) -> Self {
        // Get the base address of the objects
        // and use the properties addr from the first object to find the end of the object table
        let mut base = 0;
        let mut objs = vec![];

        let object_table_offset = gfile.object_table();
        let raw_object_bytes = &gfile.bytes()[object_table_offset..];

        // Remarks
        // The largest valid object number is not directly stored anywhere in the Z-machine.
        // Utility programs like Infodump deduce this number by assuming that, initially,
        // the object entries end where the first property table begins.
        let prop_base_offset = Zobject::properties_addr_from_base(raw_object_bytes);
        let obj_table_size = prop_base_offset - object_table_offset;

        // usual calculation of number of objects based on the number of bytes divided
        // by the size of each object struct
        let mut n_obj = obj_table_size / SIZE_OF_ZOBJ;

        while n_obj > 0 {
            let zobj = Zobject::new(gfile, &raw_object_bytes[base..base + SIZE_OF_ZOBJ]);
            objs.push(zobj);
            n_obj -= 1;
            base += SIZE_OF_ZOBJ;
        }

        ObjectTable { objects: objs }
    }

    /// Debug dump a specific object by number (1-based)
    pub fn debug_dump_object(&self, obj_num: usize) {
        if obj_num == 0 || obj_num > self.objects.len() {
            debug!(
                "Invalid object number: {} (valid range: 1-{})",
                obj_num,
                self.objects.len()
            );
            return;
        }

        let obj = &self.objects[obj_num - 1];
        obj.debug_dump(obj_num);
    }

    /// Debug dump all objects
    pub fn debug_dump_all(&self) {
        info!("=== Object Table Dump ===");
        info!("Total objects: {}", self.objects.len());
        for (i, obj) in self.objects.iter().enumerate() {
            obj.debug_dump(i + 1);
        }
    }
}

impl Display for ObjectTable {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let iter = self.into_iter();
        writeln!(f, "\nThere are {} objects.", iter.len())?;
        for (i, x) in iter.enumerate() {
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

impl<'a> IntoIterator for &'a ObjectTable {
    type Item = &'a Zobject;
    type IntoIter = ZObjectIntoIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ZObjectIntoIterator {
            objects: self.objects.iter(),
        }
    }
}

pub struct ZObjectIntoIterator<'a> {
    objects: std::slice::Iter<'a, Zobject>,
}

impl<'a> Iterator for ZObjectIntoIterator<'a> {
    type Item = &'a Zobject;

    fn next(&mut self) -> Option<Self::Item> {
        self.objects.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.objects.size_hint()
    }
}

impl<'a> ExactSizeIterator for ZObjectIntoIterator<'a> {}

// NOTE: this is only up to v3
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct InnerZobjectV3 {
    pub attribute_bits: [u8; 4],
    pub parent: u8,
    pub next: u8,
    pub child: u8,
    pub properties_offsets: [u8; 2],
}

const SIZE_OF_ZOBJ: usize = std::mem::size_of::<InnerZobjectV3>();
#[derive(Debug, Clone)]
pub struct Zobject {
    zobj: InnerZobjectV3,
    description: String,
    properties: Vec<(u8, Vec<u8>)>,
}

fn inner_zobject_v3_from_bytes(bytes: &[u8]) -> InnerZobjectV3 {
    unsafe {
        *std::mem::transmute::<&[u8; SIZE_OF_ZOBJ], &InnerZobjectV3>(bytes.try_into().unwrap())
    }
}

impl Zobject {
    /// create a new Zobject by bitblt'ing into InnerZobjectV3
    pub fn new(gfile: &GameFile, bytes: &[u8]) -> Zobject {
        let zobj = inner_zobject_v3_from_bytes(&bytes[0..SIZE_OF_ZOBJ]);

        let properties_addr = u16::from_be_bytes(zobj.properties_offsets) as usize;
        let descr_byte_len: usize = gfile.bytes()[properties_addr] as usize;
        // This next line checks for zero-length description
        let description = if descr_byte_len == 0 {
            "".to_string()
        }
        // if we have a description, we read and expand abbrevs
        else {
            "OBJECT_DESC".to_string() // Temporary placeholder
        };

        // also read properties into object, starting at
        // properties_addr + 1 for the byte denoting description len + the actual description len, which is in 2 byte words
        let mut properties_base = properties_addr + 1 + descr_byte_len * 2;
        let mut props = vec![];
        loop {
            let property_size_byte = gfile.bytes()[properties_base];
            // BUGBUG?
            /*12.4.2.1.1
            ***[1.0] A value of 0 as property data length (in the second byte) should be interpreted as a length of 64.
            (Inform can compile such properties.) */
            if property_size_byte == 0 {
                break;
            } else {
                let actual_size = (property_size_byte >> 5) + 1;
                let property_index = property_size_byte & 0b00011111;
                let mut prop_bytes = vec![];
                for i in 0..actual_size {
                    prop_bytes.push(gfile.bytes()[properties_base + 1 + i as usize]);
                }
                props.push((property_index, prop_bytes));
                properties_base += (actual_size + 1) as usize;
            }
        }

        Zobject {
            zobj,
            description,
            properties: props,
        }
    }

    /// return object's attributes
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

    /*
       12.4.1
       In Versions 1 to 3, each property is stored as a block
          size byte     the actual property data
                       ---between 1 and 8 bytes--
       where the size byte is arranged as 32 times the number of data bytes minus one, plus the property number. A property list is terminated by a size byte of 0. (It is otherwise illegal for a size byte to be a multiple of 32.)
       12.4.2
       In Versions 4 and later, a property block instead has the form
          size and number       the actual property data
         --1 or 2 bytes---     --between 1 and 64 bytes--
       The property number occupies the bottom 6 bits of the first size byte.
    */

    /// return properties offset from object's data
    pub fn properties_addr(&self) -> usize {
        u16::from_be_bytes(self.zobj.properties_offsets) as usize
    }

    /// given a pointer to object memory, return its properties
    /// used to find the end of the object table (where the properties begin)
    /// Each object in the object table has properties that follow in a properties table,
    /// so the number of objects is ((property table start) - (object table start)) / sizeof(object)
    pub fn properties_addr_from_base(bytes: &[u8]) -> usize {
        let zobj = inner_zobject_v3_from_bytes(&bytes[0..SIZE_OF_ZOBJ]);

        u16::from_be_bytes(zobj.properties_offsets) as usize
    }

    /// Debug dump this object with detailed information
    pub fn debug_dump(&self, obj_num: usize) {
        debug!("=== Object #{} ===", obj_num);

        // Attributes (32 bits)
        let attrs = self.attributes();
        debug!("  Attributes: {:?}", attrs);
        let mut set_attrs = Vec::new();
        for (i, &byte) in attrs.iter().enumerate() {
            for bit in 0..8 {
                if byte & (1 << (7 - bit)) != 0 {
                    set_attrs.push(i * 8 + bit);
                }
            }
        }
        if !set_attrs.is_empty() {
            debug!("  Set attributes: {:?}", set_attrs);
        }

        // Object relationships
        debug!("  Parent: {}", self.zobj.parent);
        debug!("  Sibling: {}", self.zobj.next);
        debug!("  Child: {}", self.zobj.child);

        // Description
        debug!("  Description: \"{}\"", self.description);

        // Properties
        debug!("  Properties address: {:#06x}", self.properties_addr());
        if !self.properties.is_empty() {
            debug!("  Properties:");
            for (prop_num, data) in &self.properties {
                debug!("    Property {}: {:?} (len={})", prop_num, data, data.len());
            }
        } else {
            debug!("  No properties");
        }
    }
}

impl Display for Zobject {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(
            f,
            "Attributes: {:?}, 
            Parent object: {}, Sibling object: {}, Child object: {}, 
            Property Address {:#04x},
                Description: \"{}\",
                ",
            self.attributes(),
            self.zobj.parent,
            self.zobj.next,
            self.zobj.child,
            self.properties_addr(),
            self.description,
        )?;
        write!(f, "Properties:")?;
        for (k, v) in &self.properties {
            write!(
                f,
                "
                    [{k}]: "
            )?;
            for val in v {
                write!(f, "{val:02x} ")?;
            }
        }
        Ok(())
    }
}

pub struct ZobjectPostV3 {
    pub attributes: [u16; 3],
    pub parent: u16,
    pub next: u16,
    pub child: u16,
    pub property_offset: u16,
}

/*
1 2 3 4 5 6 7 8
1: 32 * 1 - 1 = 31 or 32 * (1-1) = 0, no bits
2: 32 * 2 -1 = 63 or 32 * (2-1) = 32, 5th bit
3: 32 * 3 - = 95 or 32 * (3-1) = 64, 6th bit
4: 32 * (4-1) = 96
5: 32 * (5-1) = 128
6: 32 * (6-1) = 160
7: 32 * (7-1) = 192
8: 32 * (8-1) = 224
*/

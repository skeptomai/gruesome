/// Z-Machine Object System for Version 3
///
/// V3 Object Format:
/// - Maximum 255 objects
/// - 32 attributes (0-31)
/// - 31 default properties
/// - 9-byte object entries
/// - Property numbers 1-31
use crate::vm::VM;
use log::debug;

pub const MAX_OBJECTS_V3: u16 = 255;
pub const MAX_ATTRIBUTES_V3: u16 = 31;
pub const MAX_PROPERTIES_V3: u16 = 31;
pub const OBJECT_ENTRY_SIZE_V3: usize = 9;

pub trait ObjectSystemV3 {
    fn get_object_addr_v3(&self, obj_num: u16) -> Result<usize, String>;
    fn get_object_parent_v3(&self, obj_num: u16) -> Result<u16, String>;
    fn set_object_parent_v3(&mut self, obj_num: u16, parent: u16) -> Result<(), String>;
    fn get_object_sibling_v3(&self, obj_num: u16) -> Result<u16, String>;
    fn set_object_sibling_v3(&mut self, obj_num: u16, sibling: u16) -> Result<(), String>;
    fn get_object_child_v3(&self, obj_num: u16) -> Result<u16, String>;
    fn set_object_child_v3(&mut self, obj_num: u16, child: u16) -> Result<(), String>;
    fn test_object_attribute_v3(&self, obj_num: u16, attr_num: u16) -> Result<bool, String>;
    fn set_object_attribute_v3(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String>;
    fn clear_object_attribute_v3(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String>;
    fn get_object_property_v3(&self, obj_num: u16, prop_num: u16) -> Result<u16, String>;
    fn set_object_property_v3(
        &mut self,
        obj_num: u16,
        prop_num: u16,
        value: u16,
    ) -> Result<(), String>;
    fn get_object_property_addr_v3(&self, obj_num: u16, prop_num: u16) -> Result<u16, String>;
    fn get_next_object_property_v3(&self, obj_num: u16, prop_num: u16) -> Result<u16, String>;
}

impl ObjectSystemV3 for VM {
    fn get_object_addr_v3(&self, obj_num: u16) -> Result<usize, String> {
        if obj_num == 0 || obj_num > MAX_OBJECTS_V3 {
            return Err(format!(
                "Invalid v3 object number: {obj_num} (max: {MAX_OBJECTS_V3})"
            ));
        }

        let obj_table_addr = self.game.header.object_table_addr;
        let property_defaults = obj_table_addr;
        let obj_tree_base = property_defaults + MAX_PROPERTIES_V3 as usize * 2;

        Ok(obj_tree_base + ((obj_num - 1) as usize * OBJECT_ENTRY_SIZE_V3))
    }

    fn get_object_parent_v3(&self, obj_num: u16) -> Result<u16, String> {
        let obj_addr = self.get_object_addr_v3(obj_num)?;
        Ok(self.game.memory[obj_addr + 4] as u16)
    }

    fn set_object_parent_v3(&mut self, obj_num: u16, parent: u16) -> Result<(), String> {
        if parent > MAX_OBJECTS_V3 {
            return Err(format!(
                "Parent object number too large for v3: {parent} (max: {MAX_OBJECTS_V3})"
            ));
        }
        let obj_addr = self.get_object_addr_v3(obj_num)?;
        self.game.memory[obj_addr + 4] = parent as u8;
        Ok(())
    }

    fn get_object_sibling_v3(&self, obj_num: u16) -> Result<u16, String> {
        let obj_addr = self.get_object_addr_v3(obj_num)?;
        Ok(self.game.memory[obj_addr + 5] as u16)
    }

    fn set_object_sibling_v3(&mut self, obj_num: u16, sibling: u16) -> Result<(), String> {
        if sibling > MAX_OBJECTS_V3 {
            return Err(format!(
                "Sibling object number too large for v3: {sibling} (max: {MAX_OBJECTS_V3})"
            ));
        }
        let obj_addr = self.get_object_addr_v3(obj_num)?;
        self.game.memory[obj_addr + 5] = sibling as u8;
        Ok(())
    }

    fn get_object_child_v3(&self, obj_num: u16) -> Result<u16, String> {
        let obj_addr = self.get_object_addr_v3(obj_num)?;
        Ok(self.game.memory[obj_addr + 6] as u16)
    }

    fn set_object_child_v3(&mut self, obj_num: u16, child: u16) -> Result<(), String> {
        if child > MAX_OBJECTS_V3 {
            return Err(format!(
                "Child object number too large for v3: {child} (max: {MAX_OBJECTS_V3})"
            ));
        }
        let obj_addr = self.get_object_addr_v3(obj_num)?;
        self.game.memory[obj_addr + 6] = child as u8;
        Ok(())
    }

    fn test_object_attribute_v3(&self, obj_num: u16, attr_num: u16) -> Result<bool, String> {
        if attr_num > MAX_ATTRIBUTES_V3 {
            debug!("Warning: Attribute {attr_num} out of range for v3 (max: {MAX_ATTRIBUTES_V3})");
            return Ok(false);
        }

        let obj_addr = self.get_object_addr_v3(obj_num)?;
        let byte_offset = attr_num / 8;
        let bit_offset = 7 - (attr_num % 8);
        let attr_byte = self.game.memory[obj_addr + byte_offset as usize];

        Ok((attr_byte & (1 << bit_offset)) != 0)
    }

    fn set_object_attribute_v3(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String> {
        if attr_num > MAX_ATTRIBUTES_V3 {
            debug!("Warning: Trying to set attribute {attr_num} out of range for v3 (max: {MAX_ATTRIBUTES_V3})");
            return Ok(());
        }

        let obj_addr = self.get_object_addr_v3(obj_num)?;
        let byte_offset = attr_num / 8;
        let bit_offset = 7 - (attr_num % 8);
        let byte_addr = obj_addr + byte_offset as usize;

        self.game.memory[byte_addr] |= 1 << bit_offset;
        Ok(())
    }

    fn clear_object_attribute_v3(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String> {
        if attr_num > MAX_ATTRIBUTES_V3 {
            debug!("Warning: Trying to clear attribute {attr_num} out of range for v3 (max: {MAX_ATTRIBUTES_V3})");
            return Ok(());
        }

        let obj_addr = self.get_object_addr_v3(obj_num)?;
        let byte_offset = attr_num / 8;
        let bit_offset = 7 - (attr_num % 8);
        let byte_addr = obj_addr + byte_offset as usize;

        self.game.memory[byte_addr] &= !(1 << bit_offset);
        Ok(())
    }

    fn get_object_property_v3(&self, obj_num: u16, prop_num: u16) -> Result<u16, String> {
        if prop_num == 0 {
            return Err("Property number 0 is invalid".to_string());
        }

        // Get property table address
        let obj_addr = self.get_object_addr_v3(obj_num)?;
        let prop_table_addr = self.read_word((obj_addr + 7) as u32) as usize;

        // Skip object name (ZSTRING)
        let text_len = self.game.memory[prop_table_addr];
        let mut prop_addr = prop_table_addr + 1 + (text_len as usize * 2);

        // Search for property
        loop {
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                break; // End of properties
            }

            // V3: prop num in bottom 5 bits, size in top 3 bits
            let current_prop_num = size_byte & 0x1F;
            let prop_size = ((size_byte >> 5) & 0x07) + 1;

            if current_prop_num == prop_num as u8 {
                // Found the property
                prop_addr += 1;
                return match prop_size {
                    1 => Ok(self.game.memory[prop_addr] as u16),
                    2 => Ok(self.read_word(prop_addr as u32)),
                    _ => Err(format!("Invalid property size in v3: {prop_size}")),
                };
            }

            if current_prop_num < prop_num as u8 {
                break; // Properties are in descending order
            }

            prop_addr += 1 + prop_size as usize;
        }

        // Property not found, return default value
        if prop_num <= MAX_PROPERTIES_V3 {
            let obj_table_addr = self.game.header.object_table_addr;
            let default_addr = obj_table_addr + ((prop_num - 1) * 2) as usize;
            Ok(self.read_word(default_addr as u32))
        } else {
            Ok(0)
        }
    }

    fn set_object_property_v3(
        &mut self,
        obj_num: u16,
        prop_num: u16,
        value: u16,
    ) -> Result<(), String> {
        if prop_num == 0 {
            return Err("Property number 0 is invalid".to_string());
        }

        // Get property table address
        let obj_addr = self.get_object_addr_v3(obj_num)?;
        let prop_table_addr = self.read_word((obj_addr + 7) as u32) as usize;

        // Skip object name (ZSTRING)
        let text_len = self.game.memory[prop_table_addr];
        let mut prop_addr = prop_table_addr + 1 + (text_len as usize * 2);

        // Search for property
        loop {
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                return Err(format!("Property {prop_num} not found in object {obj_num}"));
            }

            // V3: prop num in bottom 5 bits, size in top 3 bits
            let current_prop_num = size_byte & 0x1F;
            let prop_size = ((size_byte >> 5) & 0x07) + 1;

            if current_prop_num == prop_num as u8 {
                // Found the property
                prop_addr += 1;
                match prop_size {
                    1 => {
                        if value > 255 {
                            return Err(format!("Value {value} too large for 1-byte property"));
                        }
                        self.game.memory[prop_addr] = value as u8;
                    }
                    2 => {
                        self.write_word(prop_addr as u32, value)?;
                    }
                    _ => return Err(format!("Invalid property size in v3: {prop_size}")),
                }
                return Ok(());
            }

            if current_prop_num < prop_num as u8 {
                return Err(format!("Property {prop_num} not found in object {obj_num}"));
            }

            prop_addr += 1 + prop_size as usize;
        }
    }

    fn get_object_property_addr_v3(&self, obj_num: u16, prop_num: u16) -> Result<u16, String> {
        if prop_num == 0 {
            return Ok(0);
        }

        // Get property table address
        let obj_addr = self.get_object_addr_v3(obj_num)?;
        let prop_table_addr = self.read_word((obj_addr + 7) as u32) as usize;

        // Skip object name (ZSTRING)
        let text_len = self.game.memory[prop_table_addr];
        let mut prop_addr = prop_table_addr + 1 + (text_len as usize * 2);

        // Search for property
        loop {
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                return Ok(0); // Property not found
            }

            // V3: prop num in bottom 5 bits
            let current_prop_num = size_byte & 0x1F;

            if current_prop_num == prop_num as u8 {
                return Ok((prop_addr + 1) as u16); // Return address of property data
            }

            if current_prop_num < prop_num as u8 {
                return Ok(0); // Properties are in descending order
            }

            let prop_size = ((size_byte >> 5) & 0x07) + 1;
            prop_addr += 1 + prop_size as usize;
        }
    }

    fn get_next_object_property_v3(&self, obj_num: u16, prop_num: u16) -> Result<u16, String> {
        // Get property table address
        let obj_addr = self.get_object_addr_v3(obj_num)?;
        let prop_table_addr = self.read_word((obj_addr + 7) as u32) as usize;

        // Skip object name (ZSTRING)
        let text_len = self.game.memory[prop_table_addr];
        let mut prop_addr = prop_table_addr + 1 + (text_len as usize * 2);

        if prop_num == 0 {
            // Return first property
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                return Ok(0); // No properties
            }
            return Ok((size_byte & 0x1F) as u16);
        }

        // Find the specified property and return the next one
        loop {
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                return Ok(0); // End of properties
            }

            let current_prop_num = size_byte & 0x1F;
            let prop_size = ((size_byte >> 5) & 0x07) + 1;

            if current_prop_num == prop_num as u8 {
                // Found current property, move to next
                prop_addr += 1 + prop_size as usize;
                let next_size_byte = self.game.memory[prop_addr];
                if next_size_byte == 0 {
                    return Ok(0); // No next property
                }
                return Ok((next_size_byte & 0x1F) as u16);
            }

            prop_addr += 1 + prop_size as usize;
        }
    }
}

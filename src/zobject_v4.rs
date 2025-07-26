/// Z-Machine Object System for Version 4+
///
/// V4+ Object Format:
/// - Maximum 65535 objects
/// - 48 attributes (0-47)  
/// - 63 default properties
/// - 14-byte object entries
/// - Property numbers 1-63
use crate::vm::VM;
use log::debug;

pub const MAX_OBJECTS_V4: u16 = 65535;
pub const MAX_ATTRIBUTES_V4: u16 = 47;
pub const MAX_PROPERTIES_V4: u16 = 63;
pub const OBJECT_ENTRY_SIZE_V4: usize = 14;

pub trait ObjectSystemV4 {
    fn get_object_addr_v4(&self, obj_num: u16) -> Result<usize, String>;
    fn get_object_parent_v4(&self, obj_num: u16) -> Result<u16, String>;
    fn set_object_parent_v4(&mut self, obj_num: u16, parent: u16) -> Result<(), String>;
    fn get_object_sibling_v4(&self, obj_num: u16) -> Result<u16, String>;
    fn set_object_sibling_v4(&mut self, obj_num: u16, sibling: u16) -> Result<(), String>;
    fn get_object_child_v4(&self, obj_num: u16) -> Result<u16, String>;
    fn set_object_child_v4(&mut self, obj_num: u16, child: u16) -> Result<(), String>;
    fn test_object_attribute_v4(&self, obj_num: u16, attr_num: u16) -> Result<bool, String>;
    fn set_object_attribute_v4(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String>;
    fn clear_object_attribute_v4(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String>;
    fn get_object_property_v4(&self, obj_num: u16, prop_num: u16) -> Result<u16, String>;
    fn set_object_property_v4(
        &mut self,
        obj_num: u16,
        prop_num: u16,
        value: u16,
    ) -> Result<(), String>;
    fn get_object_property_addr_v4(&self, obj_num: u16, prop_num: u16) -> Result<u16, String>;
    fn get_next_object_property_v4(&self, obj_num: u16, prop_num: u16) -> Result<u16, String>;
}

impl ObjectSystemV4 for VM {
    fn get_object_addr_v4(&self, obj_num: u16) -> Result<usize, String> {
        if obj_num == 0 {
            return Err(format!(
                "Invalid v4+ object number: {obj_num} (max: {MAX_OBJECTS_V4})"
            ));
        }

        let obj_table_addr = self.game.header.object_table_addr;
        let property_defaults = obj_table_addr;
        let obj_tree_base = property_defaults + MAX_PROPERTIES_V4 as usize * 2;

        Ok(obj_tree_base + ((obj_num - 1) as usize * OBJECT_ENTRY_SIZE_V4))
    }

    fn get_object_parent_v4(&self, obj_num: u16) -> Result<u16, String> {
        let obj_addr = self.get_object_addr_v4(obj_num)?;
        Ok(self.read_word((obj_addr + 6) as u32))
    }

    fn set_object_parent_v4(&mut self, obj_num: u16, parent: u16) -> Result<(), String> {
        let obj_addr = self.get_object_addr_v4(obj_num)?;
        self.write_word((obj_addr + 6) as u32, parent)?;
        Ok(())
    }

    fn get_object_sibling_v4(&self, obj_num: u16) -> Result<u16, String> {
        let obj_addr = self.get_object_addr_v4(obj_num)?;
        Ok(self.read_word((obj_addr + 8) as u32))
    }

    fn set_object_sibling_v4(&mut self, obj_num: u16, sibling: u16) -> Result<(), String> {
        let obj_addr = self.get_object_addr_v4(obj_num)?;
        self.write_word((obj_addr + 8) as u32, sibling)?;
        Ok(())
    }

    fn get_object_child_v4(&self, obj_num: u16) -> Result<u16, String> {
        let obj_addr = self.get_object_addr_v4(obj_num)?;
        Ok(self.read_word((obj_addr + 10) as u32))
    }

    fn set_object_child_v4(&mut self, obj_num: u16, child: u16) -> Result<(), String> {
        let obj_addr = self.get_object_addr_v4(obj_num)?;
        self.write_word((obj_addr + 10) as u32, child)?;
        Ok(())
    }

    fn test_object_attribute_v4(&self, obj_num: u16, attr_num: u16) -> Result<bool, String> {
        if attr_num > MAX_ATTRIBUTES_V4 {
            debug!("Warning: Attribute {attr_num} out of range for v4+ (max: {MAX_ATTRIBUTES_V4})");
            return Ok(false);
        }

        let obj_addr = self.get_object_addr_v4(obj_num)?;
        let byte_offset = attr_num / 8;
        let bit_offset = 7 - (attr_num % 8);
        let attr_byte = self.game.memory[obj_addr + byte_offset as usize];

        Ok((attr_byte & (1 << bit_offset)) != 0)
    }

    fn set_object_attribute_v4(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String> {
        if attr_num > MAX_ATTRIBUTES_V4 {
            debug!("Warning: Trying to set attribute {attr_num} out of range for v4+ (max: {MAX_ATTRIBUTES_V4})");
            return Ok(());
        }

        let obj_addr = self.get_object_addr_v4(obj_num)?;
        let byte_offset = attr_num / 8;
        let bit_offset = 7 - (attr_num % 8);
        let byte_addr = obj_addr + byte_offset as usize;

        self.game.memory[byte_addr] |= 1 << bit_offset;
        Ok(())
    }

    fn clear_object_attribute_v4(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String> {
        if attr_num > MAX_ATTRIBUTES_V4 {
            debug!("Warning: Trying to clear attribute {attr_num} out of range for v4+ (max: {MAX_ATTRIBUTES_V4})");
            return Ok(());
        }

        let obj_addr = self.get_object_addr_v4(obj_num)?;
        let byte_offset = attr_num / 8;
        let bit_offset = 7 - (attr_num % 8);
        let byte_addr = obj_addr + byte_offset as usize;

        self.game.memory[byte_addr] &= !(1 << bit_offset);
        Ok(())
    }

    fn get_object_property_v4(&self, obj_num: u16, prop_num: u16) -> Result<u16, String> {
        if prop_num == 0 {
            return Err("Property number 0 is invalid".to_string());
        }

        // Get property table address
        let obj_addr = self.get_object_addr_v4(obj_num)?;
        let prop_table_addr = self.read_word((obj_addr + 12) as u32) as usize;

        // Skip object name (ZSTRING)
        let text_len = self.game.memory[prop_table_addr];
        let mut prop_addr = prop_table_addr + 1 + (text_len as usize * 2);

        // Search for property
        loop {
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                break; // End of properties
            }

            // V4+: Different property format
            let (current_prop_num, prop_size) = if (size_byte & 0x80) != 0 {
                // Two-byte format: first byte has bit 7 set
                let prop_num = size_byte & 0x3F;
                let second_byte = self.game.memory[prop_addr + 1];
                let size = if (size_byte & 0x40) != 0 {
                    // Size in second byte & 0x3F (can be 0-63, 0 means 64)
                    let size = second_byte & 0x3F;
                    if size == 0 {
                        64
                    } else {
                        size as usize
                    }
                } else {
                    // Size = second byte & 0x3F (but limited to reasonable values)
                    let size = second_byte & 0x3F;
                    if size == 0 {
                        64
                    } else {
                        size as usize
                    }
                };
                (prop_num, size)
            } else {
                // One-byte format: bit 7 clear
                let prop_num = size_byte & 0x3F;
                let size = if (size_byte & 0x40) != 0 { 2 } else { 1 };
                (prop_num, size)
            };

            if current_prop_num == prop_num as u8 {
                // Found the property
                let data_addr = if (size_byte & 0x80) != 0 {
                    prop_addr + 2 // Two-byte header
                } else {
                    prop_addr + 1 // One-byte header
                };

                return match prop_size {
                    1 => Ok(self.game.memory[data_addr] as u16),
                    2 => Ok(self.read_word(data_addr as u32)),
                    _ => {
                        // For larger properties, return first word
                        Ok(self.read_word(data_addr as u32))
                    }
                };
            }

            if current_prop_num < prop_num as u8 {
                break; // Properties are in descending order
            }

            // Move to next property
            let header_size = if (size_byte & 0x80) != 0 { 2 } else { 1 };
            prop_addr += header_size + prop_size;
        }

        // Property not found, return default value
        if prop_num <= MAX_PROPERTIES_V4 {
            let obj_table_addr = self.game.header.object_table_addr;
            let default_addr = obj_table_addr + ((prop_num - 1) * 2) as usize;
            Ok(self.read_word(default_addr as u32))
        } else {
            Ok(0)
        }
    }

    fn set_object_property_v4(
        &mut self,
        obj_num: u16,
        prop_num: u16,
        value: u16,
    ) -> Result<(), String> {
        if prop_num == 0 {
            return Err("Property number 0 is invalid".to_string());
        }

        // Get property table address
        let obj_addr = self.get_object_addr_v4(obj_num)?;
        let prop_table_addr = self.read_word((obj_addr + 12) as u32) as usize;

        // Skip object name (ZSTRING)
        let text_len = self.game.memory[prop_table_addr];
        let mut prop_addr = prop_table_addr + 1 + (text_len as usize * 2);

        // Search for property
        loop {
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                return Err(format!("Property {prop_num} not found in object {obj_num}"));
            }

            // V4+: Parse property format
            let (current_prop_num, prop_size) = if (size_byte & 0x80) != 0 {
                // Two-byte format
                let prop_num = size_byte & 0x3F;
                let second_byte = self.game.memory[prop_addr + 1];
                let size = second_byte & 0x3F;
                let size = if size == 0 { 64 } else { size as usize };
                (prop_num, size)
            } else {
                // One-byte format
                let prop_num = size_byte & 0x3F;
                let size = if (size_byte & 0x40) != 0 { 2 } else { 1 };
                (prop_num, size)
            };

            if current_prop_num == prop_num as u8 {
                // Found the property
                let data_addr = if (size_byte & 0x80) != 0 {
                    prop_addr + 2 // Two-byte header
                } else {
                    prop_addr + 1 // One-byte header
                };

                match prop_size {
                    1 => {
                        if value > 255 {
                            return Err(format!("Value {value} too large for 1-byte property"));
                        }
                        self.game.memory[data_addr] = value as u8;
                    }
                    2 => {
                        self.write_word(data_addr as u32, value)?;
                    }
                    _ => {
                        // For larger properties, set first word
                        self.write_word(data_addr as u32, value)?;
                    }
                }
                return Ok(());
            }

            if current_prop_num < prop_num as u8 {
                return Err(format!("Property {prop_num} not found in object {obj_num}"));
            }

            // Move to next property
            let header_size = if (size_byte & 0x80) != 0 { 2 } else { 1 };
            prop_addr += header_size + prop_size;
        }
    }

    fn get_object_property_addr_v4(&self, obj_num: u16, prop_num: u16) -> Result<u16, String> {
        if prop_num == 0 {
            return Ok(0);
        }

        // Get property table address
        let obj_addr = self.get_object_addr_v4(obj_num)?;
        let prop_table_addr = self.read_word((obj_addr + 12) as u32) as usize;

        // Skip object name (ZSTRING)
        let text_len = self.game.memory[prop_table_addr];
        let mut prop_addr = prop_table_addr + 1 + (text_len as usize * 2);

        // Search for property
        loop {
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                return Ok(0); // Property not found
            }

            // V4+: Parse property number
            let current_prop_num = if (size_byte & 0x80) != 0 {
                size_byte & 0x3F // Two-byte format
            } else {
                size_byte & 0x3F // One-byte format
            };

            if current_prop_num == prop_num as u8 {
                // Return address of property data
                let data_addr = if (size_byte & 0x80) != 0 {
                    prop_addr + 2 // Two-byte header
                } else {
                    prop_addr + 1 // One-byte header
                };
                return Ok(data_addr as u16);
            }

            if current_prop_num < prop_num as u8 {
                return Ok(0); // Properties are in descending order
            }

            // Calculate property size and move to next
            let prop_size = if (size_byte & 0x80) != 0 {
                // Two-byte format
                let second_byte = self.game.memory[prop_addr + 1];
                let size = second_byte & 0x3F;
                if size == 0 {
                    64
                } else {
                    size as usize
                }
            } else {
                // One-byte format
                if (size_byte & 0x40) != 0 {
                    2
                } else {
                    1
                }
            };

            let header_size = if (size_byte & 0x80) != 0 { 2 } else { 1 };
            prop_addr += header_size + prop_size;
        }
    }

    fn get_next_object_property_v4(&self, obj_num: u16, prop_num: u16) -> Result<u16, String> {
        // Get property table address
        let obj_addr = self.get_object_addr_v4(obj_num)?;
        let prop_table_addr = self.read_word((obj_addr + 12) as u32) as usize;

        // Skip object name (ZSTRING)
        let text_len = self.game.memory[prop_table_addr];
        let mut prop_addr = prop_table_addr + 1 + (text_len as usize * 2);

        if prop_num == 0 {
            // Return first property
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                return Ok(0); // No properties
            }
            let first_prop_num = if (size_byte & 0x80) != 0 {
                size_byte & 0x3F // Two-byte format
            } else {
                size_byte & 0x3F // One-byte format
            };
            return Ok(first_prop_num as u16);
        }

        // Find the specified property and return the next one
        loop {
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                return Ok(0); // End of properties
            }

            let current_prop_num = if (size_byte & 0x80) != 0 {
                size_byte & 0x3F // Two-byte format
            } else {
                size_byte & 0x3F // One-byte format
            };

            if current_prop_num == prop_num as u8 {
                // Found current property, move to next
                let prop_size = if (size_byte & 0x80) != 0 {
                    // Two-byte format
                    let second_byte = self.game.memory[prop_addr + 1];
                    let size = if (size_byte & 0x40) != 0 {
                        let size = second_byte & 0x3F;
                        if size == 0 {
                            64
                        } else {
                            size as usize
                        }
                    } else {
                        let size = second_byte & 0x3F;
                        if size == 0 {
                            64
                        } else {
                            size as usize
                        }
                    };
                    size
                } else {
                    // One-byte format
                    if (size_byte & 0x40) != 0 {
                        2
                    } else {
                        1
                    }
                };

                let header_size = if (size_byte & 0x80) != 0 { 2 } else { 1 };
                prop_addr += header_size + prop_size;

                let next_size_byte = self.game.memory[prop_addr];
                if next_size_byte == 0 {
                    return Ok(0); // No next property
                }

                let next_prop_num = if (next_size_byte & 0x80) != 0 {
                    next_size_byte & 0x3F // Two-byte format
                } else {
                    next_size_byte & 0x3F // One-byte format
                };
                return Ok(next_prop_num as u16);
            }

            // Move to next property
            let prop_size = if (size_byte & 0x80) != 0 {
                // Two-byte format
                let second_byte = self.game.memory[prop_addr + 1];
                let size = second_byte & 0x3F;
                if size == 0 {
                    64
                } else {
                    size as usize
                }
            } else {
                // One-byte format
                if (size_byte & 0x40) != 0 {
                    2
                } else {
                    1
                }
            };

            let header_size = if (size_byte & 0x80) != 0 { 2 } else { 1 };
            prop_addr += header_size + prop_size;
        }
    }
}

/// Unified Z-Machine Object System Interface
///
/// This module provides a version-agnostic interface that dispatches
/// to the appropriate v3 or v4+ object system implementation.
use crate::interpreter::core::vm::VM;
use crate::interpreter::objects::zobject_v3::ObjectSystemV3;
use crate::interpreter::objects::zobject_v4::ObjectSystemV4;

pub trait ZObjectSystem {
    fn get_object_parent(&self, obj_num: u16) -> Result<u16, String>;
    fn set_object_parent(&mut self, obj_num: u16, parent: u16) -> Result<(), String>;
    fn get_object_sibling(&self, obj_num: u16) -> Result<u16, String>;
    fn set_object_sibling(&mut self, obj_num: u16, sibling: u16) -> Result<(), String>;
    fn get_object_child(&self, obj_num: u16) -> Result<u16, String>;
    fn set_object_child(&mut self, obj_num: u16, child: u16) -> Result<(), String>;
    fn test_object_attribute(&self, obj_num: u16, attr_num: u16) -> Result<bool, String>;
    fn set_object_attribute(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String>;
    fn clear_object_attribute(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String>;
    fn get_object_property(&self, obj_num: u16, prop_num: u16) -> Result<u16, String>;
    fn set_object_property(
        &mut self,
        obj_num: u16,
        prop_num: u16,
        value: u16,
    ) -> Result<(), String>;
    fn get_object_property_addr(&self, obj_num: u16, prop_num: u16) -> Result<u16, String>;
    fn get_next_object_property(&self, obj_num: u16, prop_num: u16) -> Result<u16, String>;
}

impl ZObjectSystem for VM {
    fn get_object_parent(&self, obj_num: u16) -> Result<u16, String> {
        if self.game.header.version <= 3 {
            self.get_object_parent_v3(obj_num)
        } else {
            self.get_object_parent_v4(obj_num)
        }
    }

    fn set_object_parent(&mut self, obj_num: u16, parent: u16) -> Result<(), String> {
        if self.game.header.version <= 3 {
            self.set_object_parent_v3(obj_num, parent)
        } else {
            self.set_object_parent_v4(obj_num, parent)
        }
    }

    fn get_object_sibling(&self, obj_num: u16) -> Result<u16, String> {
        if self.game.header.version <= 3 {
            self.get_object_sibling_v3(obj_num)
        } else {
            self.get_object_sibling_v4(obj_num)
        }
    }

    fn set_object_sibling(&mut self, obj_num: u16, sibling: u16) -> Result<(), String> {
        if self.game.header.version <= 3 {
            self.set_object_sibling_v3(obj_num, sibling)
        } else {
            self.set_object_sibling_v4(obj_num, sibling)
        }
    }

    fn get_object_child(&self, obj_num: u16) -> Result<u16, String> {
        if self.game.header.version <= 3 {
            self.get_object_child_v3(obj_num)
        } else {
            self.get_object_child_v4(obj_num)
        }
    }

    fn set_object_child(&mut self, obj_num: u16, child: u16) -> Result<(), String> {
        if self.game.header.version <= 3 {
            self.set_object_child_v3(obj_num, child)
        } else {
            self.set_object_child_v4(obj_num, child)
        }
    }

    fn test_object_attribute(&self, obj_num: u16, attr_num: u16) -> Result<bool, String> {
        if self.game.header.version <= 3 {
            self.test_object_attribute_v3(obj_num, attr_num)
        } else {
            self.test_object_attribute_v4(obj_num, attr_num)
        }
    }

    fn set_object_attribute(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String> {
        if self.game.header.version <= 3 {
            self.set_object_attribute_v3(obj_num, attr_num)
        } else {
            self.set_object_attribute_v4(obj_num, attr_num)
        }
    }

    fn clear_object_attribute(&mut self, obj_num: u16, attr_num: u16) -> Result<(), String> {
        if self.game.header.version <= 3 {
            self.clear_object_attribute_v3(obj_num, attr_num)
        } else {
            self.clear_object_attribute_v4(obj_num, attr_num)
        }
    }

    fn get_object_property(&self, obj_num: u16, prop_num: u16) -> Result<u16, String> {
        if self.game.header.version <= 3 {
            self.get_object_property_v3(obj_num, prop_num)
        } else {
            self.get_object_property_v4(obj_num, prop_num)
        }
    }

    fn set_object_property(
        &mut self,
        obj_num: u16,
        prop_num: u16,
        value: u16,
    ) -> Result<(), String> {
        if self.game.header.version <= 3 {
            self.set_object_property_v3(obj_num, prop_num, value)
        } else {
            self.set_object_property_v4(obj_num, prop_num, value)
        }
    }

    fn get_object_property_addr(&self, obj_num: u16, prop_num: u16) -> Result<u16, String> {
        if self.game.header.version <= 3 {
            self.get_object_property_addr_v3(obj_num, prop_num)
        } else {
            self.get_object_property_addr_v4(obj_num, prop_num)
        }
    }

    fn get_next_object_property(&self, obj_num: u16, prop_num: u16) -> Result<u16, String> {
        if self.game.header.version <= 3 {
            self.get_next_object_property_v3(obj_num, prop_num)
        } else {
            self.get_next_object_property_v4(obj_num, prop_num)
        }
    }
}

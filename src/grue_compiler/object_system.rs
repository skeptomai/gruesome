// Comprehensive Object System for Grue Language
//
// This module provides a complete Z-Machine compatible object system with:
// - Standard named attributes mapped to Z-Machine attribute numbers
// - Standard properties mapped to Z-Machine property numbers
// - Property inheritance and defaults
// - Object type system (rooms, containers, items, etc.)

use indexmap::IndexMap;
// Removed unused import

/// Standard Z-Machine attributes with their assigned numbers
/// These follow Inform conventions for maximum compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardAttribute {
    // Core object attributes (0-15)
    Invisible = 0,   // Object cannot be seen
    Takeable = 1,    // Object can be taken by player
    Wearable = 2,    // Object can be worn
    Container = 3,   // Object can contain other objects
    Openable = 4,    // Object can be opened/closed
    Open = 5,        // Object is currently open
    Lockable = 6,    // Object can be locked/unlocked
    Locked = 7,      // Object is currently locked
    Edible = 8,      // Object can be eaten
    Drinkable = 9,   // Object can be drunk
    Pushable = 10,   // Object can be pushed
    Pullable = 11,   // Object can be pulled
    Turnable = 12,   // Object can be turned
    Switchable = 13, // Object can be switched on/off
    On = 14,         // Object is currently on
    Readable = 15,   // Object can be read

    // Game state attributes (16-31)
    Visited = 16,     // Room has been visited
    Light = 17,       // Object provides light
    Darkness = 18,    // Room is dark without light source
    Scenery = 19,     // Object is part of the scenery
    Proper = 20,      // Object name is a proper noun
    Plural = 21,      // Object name is plural
    Animate = 22,     // Object is alive/animate
    Static = 23,      // Object cannot be moved
    Concealed = 24,   // Object is hidden from view
    Transparent = 25, // Contents are visible when closed
    Supporter = 26,   // Object can support other objects
    Enterable = 27,   // Object can be entered
    Weapon = 28,      // Object is a weapon
    Treasure = 29,    // Object is a treasure (for scoring)
    Sacred = 30,      // Object cannot be taken from temple
    Worn = 31,        // Object is currently being worn
}

/// Standard Z-Machine properties with their assigned numbers
/// These follow established Z-Machine and Inform conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardProperty {
    // Core description properties (1-10)
    ShortName = 1,   // Object's short name (string)
    Description = 2, // Object's full description (string)
    InitialDesc = 3, // Initial description when first seen (string)

    // Container/supporter properties (11-20)
    Capacity = 11, // Maximum number of objects it can hold
    Size = 12,     // Size/bulk of the object
    Weight = 13,   // Weight of the object
    Value = 14,    // Monetary value of the object

    // Interactive properties (21-30)
    KeyId = 21,    // ID of key that unlocks this object
    TimeLeft = 22, // Time remaining (for timers)
    TimeOut = 23,  // Timeout routine address

    // Game mechanics (31-40)
    Points = 31, // Points awarded for taking this treasure
    Daemon = 32, // Daemon routine address

    // Room-specific properties (41-50)
    NorthExit = 41,     // Room to the north
    SouthExit = 42,     // Room to the south
    EastExit = 43,      // Room to the east
    WestExit = 44,      // Room to the west
    NortheastExit = 45, // Room to the northeast
    NorthwestExit = 46, // Room to the northwest
    SoutheastExit = 47, // Room to the southeast
    SouthwestExit = 48, // Room to the southwest
    UpExit = 49,        // Room above
    DownExit = 50,      // Room below

    // Extended directions (51-60)
    InExit = 51,  // Room/object to enter
    OutExit = 52, // Room to exit to

    // Parser properties (61-63) - maximum for Z-Machine V3
    Name = 61,    // Parser name table
    Parse = 62,   // Custom parsing routine
    Article = 63, // Article to use ("a", "an", "the", "some")
}

/// Object type classification for easier object creation
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectType {
    /// Regular item that can be taken
    Item,
    /// Container that can hold other objects
    Container {
        openable: bool,
        open: bool,
        lockable: bool,
        locked: bool,
        capacity: Option<u8>,
    },
    /// Supporter that objects can be placed on
    Supporter { capacity: Option<u8> },
    /// Room that the player can be in
    Room { light: bool },
    /// Door that connects rooms
    Door {
        openable: bool,
        open: bool,
        lockable: bool,
        locked: bool,
        key_id: Option<u8>,
    },
    /// Non-portable scenery object
    Scenery,
    /// Animate character/NPC
    Character,
    /// Light source
    LightSource { on: bool, portable: bool },
}

/// Enhanced object definition with comprehensive Z-Machine support
#[derive(Debug, Clone)]
pub struct ComprehensiveObject {
    /// Unique identifier
    pub id: String,

    /// Parser names (including multi-word names)
    pub names: Vec<String>,

    /// Object type (determines default attributes and properties)
    pub object_type: ObjectType,

    /// Z-Machine attributes (using StandardAttribute enum)
    pub attributes: IndexMap<StandardAttribute, bool>,

    /// Z-Machine numbered properties  
    pub properties: IndexMap<StandardProperty, PropertyValue>,

    /// Custom numbered properties (beyond standard ones)
    pub custom_properties: IndexMap<u8, PropertyValue>,

    /// Object hierarchy
    pub parent: Option<String>,
    pub children: Vec<String>,

    /// Location in game world
    pub location: Option<String>,
}

/// Property value types for Z-Machine properties
#[derive(Debug, Clone)]
pub enum PropertyValue {
    /// Single byte (1-255)
    Byte(u8),
    /// Word value (0-65535)
    Word(u16),
    /// Multiple bytes (up to 8 bytes in Z-Machine)
    Bytes(Vec<u8>),
    /// String reference (encoded as address)
    String(String),
    /// Object reference (encoded as object number)
    Object(String),
    /// Room reference (encoded as room number)
    Room(String),
}

impl ComprehensiveObject {
    /// Create a new object with the specified type
    pub fn new(id: String, names: Vec<String>, object_type: ObjectType) -> Self {
        let mut obj = ComprehensiveObject {
            id,
            names,
            object_type: object_type.clone(),
            attributes: IndexMap::new(),
            properties: IndexMap::new(),
            custom_properties: IndexMap::new(),
            parent: None,
            children: Vec::new(),
            location: None,
        };

        // Set default attributes and properties based on type
        obj.apply_type_defaults(&object_type);
        obj
    }

    /// Apply default attributes and properties based on object type
    fn apply_type_defaults(&mut self, object_type: &ObjectType) {
        match object_type {
            ObjectType::Item => {
                self.set_attribute(StandardAttribute::Takeable, true);
            }
            ObjectType::Container {
                openable,
                open,
                lockable,
                locked,
                capacity,
            } => {
                self.set_attribute(StandardAttribute::Container, true);
                self.set_attribute(StandardAttribute::Openable, *openable);
                self.set_attribute(StandardAttribute::Open, *open);
                self.set_attribute(StandardAttribute::Lockable, *lockable);
                self.set_attribute(StandardAttribute::Locked, *locked);
                if let Some(cap) = capacity {
                    self.set_property(StandardProperty::Capacity, PropertyValue::Byte(*cap));
                }
            }
            ObjectType::Supporter { capacity } => {
                self.set_attribute(StandardAttribute::Supporter, true);
                self.set_attribute(StandardAttribute::Static, true);
                if let Some(cap) = capacity {
                    self.set_property(StandardProperty::Capacity, PropertyValue::Byte(*cap));
                }
            }
            ObjectType::Room { light } => {
                self.set_attribute(StandardAttribute::Light, *light);
                // Rooms are not takeable and are static
                self.set_attribute(StandardAttribute::Takeable, false);
                self.set_attribute(StandardAttribute::Static, true);
            }
            ObjectType::Door {
                openable,
                open,
                lockable,
                locked,
                key_id,
            } => {
                self.set_attribute(StandardAttribute::Openable, *openable);
                self.set_attribute(StandardAttribute::Open, *open);
                self.set_attribute(StandardAttribute::Lockable, *lockable);
                self.set_attribute(StandardAttribute::Locked, *locked);
                self.set_attribute(StandardAttribute::Static, true);
                if let Some(key) = key_id {
                    self.set_property(StandardProperty::KeyId, PropertyValue::Byte(*key));
                }
            }
            ObjectType::Scenery => {
                self.set_attribute(StandardAttribute::Scenery, true);
                self.set_attribute(StandardAttribute::Static, true);
                self.set_attribute(StandardAttribute::Takeable, false);
            }
            ObjectType::Character => {
                self.set_attribute(StandardAttribute::Animate, true);
                self.set_attribute(StandardAttribute::Proper, true);
                self.set_attribute(StandardAttribute::Takeable, false);
            }
            ObjectType::LightSource { on, portable } => {
                self.set_attribute(StandardAttribute::Light, *on);
                self.set_attribute(StandardAttribute::Switchable, true);
                self.set_attribute(StandardAttribute::On, *on);
                self.set_attribute(StandardAttribute::Takeable, *portable);
            }
        }
    }

    /// Set a standard attribute
    pub fn set_attribute(&mut self, attr: StandardAttribute, value: bool) {
        self.attributes.insert(attr, value);
    }

    /// Get a standard attribute (returns false if not set)
    pub fn get_attribute(&self, attr: StandardAttribute) -> bool {
        self.attributes.get(&attr).copied().unwrap_or(false)
    }

    /// Set a standard property
    pub fn set_property(&mut self, prop: StandardProperty, value: PropertyValue) {
        self.properties.insert(prop, value);
    }

    /// Get a standard property
    pub fn get_property(&self, prop: StandardProperty) -> Option<&PropertyValue> {
        self.properties.get(&prop)
    }

    /// Set a custom numbered property
    pub fn set_custom_property(&mut self, prop_num: u8, value: PropertyValue) {
        self.custom_properties.insert(prop_num, value);
    }

    /// Convert to Z-Machine attribute bitfield
    pub fn to_zmachine_attributes(&self) -> u32 {
        let mut flags = 0u32;
        for (&attr, &value) in &self.attributes {
            if value {
                flags |= 1u32 << (attr as u8);
            }
        }
        flags
    }

    /// Generate property table for Z-Machine
    pub fn to_zmachine_properties(&self) -> Vec<(u8, PropertyValue)> {
        let mut properties = Vec::new();

        // Add standard properties
        for (&prop, value) in &self.properties {
            properties.push((prop as u8, value.clone()));
        }

        // Add custom properties
        for (&prop_num, value) in &self.custom_properties {
            properties.push((prop_num, value.clone()));
        }

        // Sort by property number (Z-Machine requires descending order)
        properties.sort_by(|a, b| b.0.cmp(&a.0));
        properties
    }
}

/// Object factory for creating common object types with defaults
pub struct ObjectFactory;

impl ObjectFactory {
    /// Create a simple takeable item
    pub fn create_item(id: String, names: Vec<String>) -> ComprehensiveObject {
        ComprehensiveObject::new(id, names, ObjectType::Item)
    }

    /// Create an openable container
    pub fn create_container(
        id: String,
        names: Vec<String>,
        openable: bool,
        capacity: Option<u8>,
    ) -> ComprehensiveObject {
        ComprehensiveObject::new(
            id,
            names,
            ObjectType::Container {
                openable,
                open: false,
                lockable: false,
                locked: false,
                capacity,
            },
        )
    }

    /// Create a room
    pub fn create_room(id: String, names: Vec<String>, has_light: bool) -> ComprehensiveObject {
        ComprehensiveObject::new(id, names, ObjectType::Room { light: has_light })
    }

    /// Create a light source
    pub fn create_light_source(
        id: String,
        names: Vec<String>,
        portable: bool,
    ) -> ComprehensiveObject {
        ComprehensiveObject::new(
            id,
            names,
            ObjectType::LightSource {
                on: false,
                portable,
            },
        )
    }

    /// Create scenery (non-takeable background object)
    pub fn create_scenery(id: String, names: Vec<String>) -> ComprehensiveObject {
        ComprehensiveObject::new(id, names, ObjectType::Scenery)
    }
}

/// Property defaults manager for inheritance
pub struct PropertyDefaults {
    defaults: IndexMap<StandardProperty, PropertyValue>,
}

impl PropertyDefaults {
    pub fn new() -> Self {
        let mut defaults = PropertyDefaults {
            defaults: IndexMap::new(),
        };
        defaults.set_standard_defaults();
        defaults
    }

    /// Set up standard property defaults
    fn set_standard_defaults(&mut self) {
        // Standard capacity defaults
        self.defaults
            .insert(StandardProperty::Capacity, PropertyValue::Byte(100));
        self.defaults
            .insert(StandardProperty::Size, PropertyValue::Byte(5));
        self.defaults
            .insert(StandardProperty::Weight, PropertyValue::Byte(5));
        self.defaults
            .insert(StandardProperty::Value, PropertyValue::Word(0));
    }

    /// Get default value for a property
    pub fn get_default(&self, prop: StandardProperty) -> Option<&PropertyValue> {
        self.defaults.get(&prop)
    }

    /// Set a default value for a property
    pub fn set_default(&mut self, prop: StandardProperty, value: PropertyValue) {
        self.defaults.insert(prop, value);
    }
}

impl Default for PropertyDefaults {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_simple_item() {
        let item = ObjectFactory::create_item(
            "lamp".to_string(),
            vec!["brass lamp".to_string(), "lamp".to_string()],
        );

        assert_eq!(item.id, "lamp");
        assert!(item.get_attribute(StandardAttribute::Takeable));
        assert!(!item.get_attribute(StandardAttribute::Container));
    }

    #[test]
    fn test_create_container() {
        let container = ObjectFactory::create_container(
            "mailbox".to_string(),
            vec![
                "small mailbox".to_string(),
                "mailbox".to_string(),
                "box".to_string(),
            ],
            true,
            Some(10),
        );

        assert!(container.get_attribute(StandardAttribute::Container));
        assert!(container.get_attribute(StandardAttribute::Openable));
        assert!(!container.get_attribute(StandardAttribute::Open));

        if let Some(PropertyValue::Byte(cap)) = container.get_property(StandardProperty::Capacity) {
            assert_eq!(*cap, 10);
        } else {
            panic!("Expected capacity property");
        }
    }

    #[test]
    fn test_zmachine_attributes() {
        let mut item = ObjectFactory::create_item("test".to_string(), vec!["test".to_string()]);
        item.set_attribute(StandardAttribute::Takeable, true);
        item.set_attribute(StandardAttribute::Light, true);

        let flags = item.to_zmachine_attributes();

        // Check bit 1 (Takeable) and bit 17 (Light) are set
        assert!(flags & (1 << 1) != 0);
        assert!(flags & (1 << 17) != 0);
    }
}

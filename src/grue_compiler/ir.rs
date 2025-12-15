// Intermediate Representation for Grue Language
//
// The IR is designed to be a lower-level representation that's closer to Z-Machine
// instructions while still maintaining some high-level constructs for optimization.

use crate::grue_compiler::ast::{ObjectSpecialization, ProgramMode, Type};
use crate::grue_compiler::object_system::ComprehensiveObject;
use indexmap::{IndexMap, IndexSet};

/// Context for expression generation to distinguish different usage patterns
/// This is critical for Z-Machine branch instruction handling
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionContext {
    /// Expression used for its value (e.g., let x = obj.open)
    /// Requires generating a boolean result value
    Value,

    /// Expression used in conditional context (e.g., if obj.open)
    /// Can use direct branch instructions for efficiency
    Conditional,

    /// Expression used in assignment context (e.g., obj.open = true)
    /// Uses set_attr/clear_attr directly
    Assignment,
}

/// Unique identifier for IR instructions, labels, and temporary variables
pub type IrId = u32;

/// Information about objects contained within a room for automatic placement
/// Used to generate InsertObj instructions during init block generation
#[derive(Debug, Clone)]
pub struct RoomObjectInfo {
    /// Name of the object (for symbol lookup)
    pub name: String,
    /// Nested objects contained within this object (e.g., leaflet inside mailbox)
    pub nested_objects: Vec<RoomObjectInfo>,
}

/// IR Program - top-level container for all IR elements
/// Registry for tracking all IR IDs and their types/purposes
#[derive(Debug, Clone)]
pub struct IrIdRegistry {
    pub id_types: IndexMap<IrId, String>,   // ID -> type description
    pub id_sources: IndexMap<IrId, String>, // ID -> creation context
    pub temporary_ids: IndexSet<IrId>,      // IDs that are temporary values
    pub symbol_ids: IndexSet<IrId>,         // IDs that are named symbols
    pub expression_ids: IndexSet<IrId>,     // IDs from expression evaluation
}

impl Default for IrIdRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl IrIdRegistry {
    pub fn new() -> Self {
        Self {
            id_types: IndexMap::new(),
            id_sources: IndexMap::new(),
            temporary_ids: IndexSet::new(),
            symbol_ids: IndexSet::new(),
            expression_ids: IndexSet::new(),
        }
    }

    pub fn register_id(&mut self, id: IrId, id_type: &str, source: &str, is_temporary: bool) {
        self.id_types.insert(id, id_type.to_string());
        self.id_sources.insert(id, source.to_string());

        if is_temporary {
            self.temporary_ids.insert(id);
        } else {
            self.symbol_ids.insert(id);
        }
    }

    pub fn register_expression_id(&mut self, id: IrId, expression_type: &str) {
        self.expression_ids.insert(id);
        self.register_id(id, expression_type, "expression", true);
    }
}

pub struct IrProgram {
    pub functions: Vec<IrFunction>,
    pub globals: Vec<IrGlobal>,
    pub rooms: Vec<IrRoom>,
    pub objects: Vec<IrObject>,
    pub grammar: Vec<IrGrammar>,
    pub init_block: Option<IrBlock>,
    pub init_block_locals: Vec<IrLocal>, // Local variables declared in init block
    pub string_table: IndexMap<String, IrId>, // String literal -> ID mapping
    pub property_defaults: IrPropertyDefaults, // Z-Machine property defaults table
    pub program_mode: ProgramMode,       // Program execution mode
    /// Mapping from symbol names to IR IDs (for identifier resolution)
    pub symbol_ids: IndexMap<String, IrId>,
    /// Mapping from object names to Z-Machine object numbers
    pub object_numbers: IndexMap<String, u16>,
    /// NEW: Comprehensive registry of all IR IDs and their purposes
    pub id_registry: IrIdRegistry,
    /// Property manager with consistent property name -> number mappings
    pub property_manager: PropertyManager,
    /// Expression type information for StringAddress system
    pub expression_types: IndexMap<IrId, Type>,
    /// System messages catalog for localization support
    /// Maps message keys to localized text (e.g., "no_understand" -> "I don't understand that.")
    pub system_messages: IndexMap<String, String>,
}

impl IrProgram {
    pub fn get_main_function(&self) -> Option<&IrFunction> {
        self.functions.iter().find(|func| func.name == "main")
    }

    /// Check if the program has any objects (rooms or objects)
    pub fn has_objects(&self) -> bool {
        !self.rooms.is_empty() || !self.objects.is_empty()
    }
}

/// Z-Machine property defaults table (31 words for V1-3, 63 for V4+)
#[derive(Debug, Clone)]
pub struct IrPropertyDefaults {
    pub defaults: IndexMap<u8, u16>, // Property number -> default value
}

impl IrPropertyDefaults {
    pub fn new() -> Self {
        Self {
            defaults: IndexMap::new(),
        }
    }

    pub fn set_default(&mut self, property_num: u8, default_value: u16) {
        self.defaults.insert(property_num, default_value);
    }

    pub fn get_default(&self, property_num: u8) -> u16 {
        self.defaults.get(&property_num).copied().unwrap_or(0)
    }

    /// Get all property defaults up to the maximum property number (for Z-Machine table generation)
    pub fn get_table(&self, max_properties: u8) -> Vec<u16> {
        let mut table = Vec::new();
        for prop_num in 1..=max_properties {
            table.push(self.get_default(prop_num));
        }
        table
    }
}

impl Default for IrPropertyDefaults {
    fn default() -> Self {
        Self::new()
    }
}

/// IR Function representation
#[derive(Debug, Clone)]
pub struct IrFunction {
    pub id: IrId,
    pub name: String,
    pub parameters: Vec<IrParameter>,
    pub return_type: Option<Type>,
    pub body: IrBlock,
    pub local_vars: Vec<IrLocal>,
}

/// Function parameter in IR
#[derive(Debug, Clone)]
pub struct IrParameter {
    pub name: String,
    pub param_type: Option<Type>,
    pub slot: u8,    // Local variable slot in Z-Machine
    pub ir_id: IrId, // IR ID for this parameter (for codegen mapping)
}

/// Local variable in IR
#[derive(Debug, Clone)]
pub struct IrLocal {
    pub ir_id: IrId, // IR ID for codegen mapping
    pub name: String,
    pub var_type: Option<Type>,
    pub slot: u8, // Local variable slot
    pub mutable: bool,
}

/// Function overload information for polymorphic dispatch
#[derive(Debug, Clone)]
pub struct FunctionOverload {
    pub function_id: IrId,
    pub specialization: ObjectSpecialization,
    pub mangled_name: String,
    pub priority: u8, // Lower number = higher priority (0 = highest)
}

/// Global variable in IR
#[derive(Debug, Clone)]
pub struct IrGlobal {
    pub id: IrId,
    pub name: String,
    pub var_type: Option<Type>,
    pub initializer: Option<IrValue>,
}

/// IR Room representation
#[derive(Debug, Clone)]
pub struct IrRoom {
    pub id: IrId,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub exits: IndexMap<String, IrExitTarget>,
    pub on_enter: Option<IrBlock>,
    pub on_exit: Option<IrBlock>,
    pub on_look: Option<IrBlock>,
}

/// IR Object representation with Z-Machine compatibility
#[derive(Debug, Clone)]
pub struct IrObject {
    pub id: IrId,
    pub name: String,
    pub short_name: String, // Z-Machine object short name (up to 765 Z-chars)
    pub names: Vec<String>, // Vocabulary names for parser
    pub description: String,
    pub attributes: IrAttributes, // Z-Machine attributes (32 for V1-3, 48 for V4+)
    pub properties: IrProperties, // Z-Machine numbered properties
    pub parent: Option<IrId>,     // Parent object or room
    pub sibling: Option<IrId>,    // Next sibling in object tree
    pub child: Option<IrId>,      // First child in object tree

    // Enhanced object system integration
    pub comprehensive_object: Option<ComprehensiveObject>, // Full object definition
}

/// Z-Machine attributes - bitflags numbered from 0
#[derive(Debug, Clone)]
pub struct IrAttributes {
    pub flags: u64, // Supports up to 48 attributes for V4+ (only 32 for V1-3)
}

impl IrAttributes {
    pub fn new() -> Self {
        Self { flags: 0 }
    }

    pub fn set(&mut self, attr: u8, value: bool) {
        if attr < 48 {
            if value {
                self.flags |= 1u64 << attr;
            } else {
                self.flags &= !(1u64 << attr);
            }
        }
    }

    pub fn get(&self, attr: u8) -> bool {
        if attr < 48 {
            (self.flags & (1u64 << attr)) != 0
        } else {
            false
        }
    }
}

impl Default for IrAttributes {
    fn default() -> Self {
        Self::new()
    }
}

/// Z-Machine properties - numbered from 1 upward
#[derive(Debug, Clone)]
pub struct IrProperties {
    pub properties: IndexMap<u8, IrPropertyValue>, // Property number -> value
}

impl IrProperties {
    pub fn new() -> Self {
        Self {
            properties: IndexMap::new(),
        }
    }

    pub fn set_byte(&mut self, prop_num: u8, value: u8) {
        self.properties
            .insert(prop_num, IrPropertyValue::Byte(value));
    }

    pub fn set_word(&mut self, prop_num: u8, value: u16) {
        self.properties
            .insert(prop_num, IrPropertyValue::Word(value));
    }

    pub fn set_bytes(&mut self, prop_num: u8, value: Vec<u8>) {
        self.properties
            .insert(prop_num, IrPropertyValue::Bytes(value));
    }

    pub fn set_string(&mut self, prop_num: u8, value: String) {
        self.properties
            .insert(prop_num, IrPropertyValue::String(value));
    }

    pub fn get(&self, prop_num: u8) -> Option<&IrPropertyValue> {
        self.properties.get(&prop_num)
    }

    pub fn get_as_word(&self, prop_num: u8) -> Option<u16> {
        match self.properties.get(&prop_num) {
            Some(IrPropertyValue::Word(value)) => Some(*value),
            Some(IrPropertyValue::Byte(value)) => Some(*value as u16),
            _ => None,
        }
    }

    pub fn has_property(&self, prop_num: u8) -> bool {
        self.properties.contains_key(&prop_num)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u8, &IrPropertyValue)> {
        self.properties.iter()
    }
}

impl Default for IrProperties {
    fn default() -> Self {
        Self::new()
    }
}

/// Property values can be 1, 2, or many bytes
#[derive(Debug, Clone)]
pub enum IrPropertyValue {
    Byte(u8),       // 1-byte property
    Word(u16),      // 2-byte property
    Bytes(Vec<u8>), // Multi-byte property
    String(String), // String property (will be converted to bytes)
}

/// Property manager for handling inheritance and dynamic property access
#[derive(Debug, Clone)]
pub struct PropertyManager {
    /// Property name to number mapping
    property_numbers: IndexMap<String, u8>,
    /// Standard property mappings
    standard_properties: IndexMap<StandardProperty, u8>,
    /// Next available property number
    next_property_number: u8,
}

impl PropertyManager {
    pub fn new() -> Self {
        let mut manager = Self {
            property_numbers: IndexMap::new(),
            standard_properties: IndexMap::new(),
            next_property_number: 1,
        };

        // Register standard properties
        manager.register_standard_property(StandardProperty::ShortName);
        manager.register_standard_property(StandardProperty::LongName);
        manager.register_standard_property(StandardProperty::Initial);
        manager.register_standard_property(StandardProperty::Before);
        manager.register_standard_property(StandardProperty::After);
        manager.register_standard_property(StandardProperty::Life);
        manager.register_standard_property(StandardProperty::Description);
        manager.register_standard_property(StandardProperty::Capacity);
        manager.register_standard_property(StandardProperty::Value);
        manager.register_standard_property(StandardProperty::Size);
        manager.register_standard_property(StandardProperty::Article);
        manager.register_standard_property(StandardProperty::Adjective);
        // Location registration removed - uses object tree only (Oct 12, 2025)

        // Pre-register exit system properties (not accessed during IR generation, only during codegen)
        // These MUST be registered here to avoid property number collisions with runtime-generated properties
        manager.get_property_number("exit_directions"); // Parallel array of dictionary addresses (2 bytes each)
        manager.get_property_number("exit_types"); // Parallel array of exit types (1 byte each): 0=room, 1=blocked
        manager.get_property_number("exit_data"); // Parallel array of exit data (2 bytes each): room_id or message_addr

        manager
    }

    fn register_standard_property(&mut self, prop: StandardProperty) {
        let prop_num = self.next_property_number;
        self.standard_properties.insert(prop, prop_num);

        let prop_name = match prop {
            StandardProperty::ShortName => "short_name",
            StandardProperty::LongName => "long_name",
            StandardProperty::Initial => "initial",
            StandardProperty::Before => "before",
            StandardProperty::After => "after",
            StandardProperty::Life => "life",
            StandardProperty::Description => "description",
            StandardProperty::Capacity => "capacity",
            StandardProperty::Value => "value",
            StandardProperty::Size => "size",
            StandardProperty::Article => "article",
            StandardProperty::Adjective => "adjective",
            // Location case removed - uses object tree only (Oct 12, 2025)
        };

        self.property_numbers
            .insert(prop_name.to_string(), prop_num);

        // Register aliases for properties that have multiple valid names
        match prop {
            StandardProperty::Description => {
                // Register "desc" as alias for "description"
                self.property_numbers.insert("desc".to_string(), prop_num);
            }
            StandardProperty::ShortName => {
                // Register "name" as alias for "short_name"
                self.property_numbers.insert("name".to_string(), prop_num);
            }
            _ => {}
        }

        self.next_property_number += 1;
    }

    pub fn get_property_number(&mut self, property_name: &str) -> u8 {
        if let Some(&existing_num) = self.property_numbers.get(property_name) {
            existing_num
        } else {
            let new_num = self.next_property_number;
            self.property_numbers
                .insert(property_name.to_string(), new_num);
            self.next_property_number += 1;
            new_num
        }
    }

    pub fn get_standard_property_number(&self, prop: StandardProperty) -> Option<u8> {
        self.standard_properties.get(&prop).copied()
    }

    /// Get all property name -> number mappings (for object table generation)
    pub fn get_property_numbers(&self) -> &IndexMap<String, u8> {
        &self.property_numbers
    }

    /// Get a specific property number by name
    pub fn get_property_number_by_name(&self, property_name: &str) -> Option<u8> {
        self.property_numbers.get(property_name).copied()
    }

    /// Get property value with inheritance from defaults
    pub fn get_property_with_inheritance(
        &self,
        object: &IrObject,
        property_num: u8,
        defaults: &IrPropertyDefaults,
    ) -> Option<IrPropertyValue> {
        // First check if object has the property directly
        if let Some(value) = object.properties.get(property_num) {
            return Some(value.clone());
        }

        // If not found, use default value
        let default_value = defaults.get_default(property_num);
        if default_value != 0 {
            Some(IrPropertyValue::Word(default_value))
        } else {
            None
        }
    }
}

impl Default for PropertyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard Z-Machine attribute definitions
#[derive(Debug, Clone, Copy)]
pub enum StandardAttribute {
    // Core object attributes (0-15) - SYNCHRONIZED with object_system.rs
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
    Moved = 16,       // Object has been moved from initial location
    Worn = 17,        // Object is being worn
    LightSource = 18, // Object provides light
    Visited = 19,     // Room has been visited
    Treasure = 20,    // Object is a treasure for scoring
    Special = 21,     // Object has special behavior
    Transparent = 22, // Can see through object to contents
    Workflag = 23,    // Temporary flag for game logic
}

/// Standard Z-Machine property numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardProperty {
    ShortName = 1,   // Object's short name (displayed name)
    LongName = 2,    // Object's long description
    Initial = 3,     // Initial room description mention
    Before = 4,      // Before routine address
    After = 5,       // After routine address
    Life = 6,        // Life routine address (for NPCs)
    Description = 7, // Room description
    Capacity = 8,    // Container capacity
    Value = 9,       // Object value for scoring
    Size = 10,       // Object size
    Article = 11,    // Article to use with object
    Adjective = 12,  // Adjectives for parsing
                     // Location removed - now uses object tree parent only (Oct 12, 2025)
}

/// Exit target in IR
#[derive(Debug, Clone)]
pub enum IrExitTarget {
    Room(IrId),
    Blocked(String),
}

/// Grammar rule in IR
#[derive(Debug, Clone)]
pub struct IrGrammar {
    pub verb: String,
    pub patterns: Vec<IrPattern>,
}

/// Grammar pattern in IR
#[derive(Debug, Clone)]
pub struct IrPattern {
    pub pattern: Vec<IrPatternElement>,
    pub handler: IrHandler,
}

/// Pattern elements in IR
#[derive(Debug, Clone, PartialEq)]
pub enum IrPatternElement {
    Literal(String),
    Noun,
    Default,

    // Enhanced parser elements for Zork I-level parsing
    Adjective,
    MultiWordNoun,
    Preposition,
    MultipleObjects,
    DirectObject,
    IndirectObject,
    OptionalAdjective,
    AnyPreposition,
    NumberedNoun,
}

/// Handler for grammar patterns
#[derive(Debug, Clone)]
pub enum IrHandler {
    FunctionCall(IrId, Vec<IrValue>), // Function ID and arguments
    Block(IrBlock),
}

/// IR Basic Block - contains sequential instructions
#[derive(Debug, Clone)]
pub struct IrBlock {
    pub id: IrId,
    pub instructions: Vec<IrInstruction>,
}

/// IR Instructions - the core IR operations
#[derive(Debug, Clone)]
pub enum IrInstruction {
    /// Load immediate value into temporary
    LoadImmediate {
        target: IrId,
        value: IrValue,
    },

    /// Load variable value into temporary
    LoadVar {
        target: IrId,
        var_id: IrId,
    },

    /// Store temporary value into variable
    StoreVar {
        var_id: IrId,
        source: IrId,
    },

    /// Binary operation
    BinaryOp {
        target: IrId,
        op: IrBinaryOp,
        left: IrId,
        right: IrId,
    },

    /// Unary operation
    UnaryOp {
        target: IrId,
        op: IrUnaryOp,
        operand: IrId,
    },

    /// Function call
    Call {
        target: Option<IrId>, // None for void functions
        function: IrId,
        args: Vec<IrId>,
    },

    /// Return from function
    Return {
        value: Option<IrId>,
    },

    /// Conditional jump
    Branch {
        condition: IrId,
        true_label: IrId,
        false_label: IrId,
    },

    /// Unconditional jump
    Jump {
        label: IrId,
    },

    /// Label (jump target)
    Label {
        id: IrId,
    },

    /// Property access
    GetProperty {
        target: IrId,
        object: IrId,
        property: String,
    },

    /// Property assignment
    SetProperty {
        object: IrId,
        property: String,
        value: IrId,
    },

    /// Numbered property access (Z-Machine style)
    GetPropertyByNumber {
        target: IrId,
        object: IrId,
        property_num: u8,
    },

    /// Numbered property assignment (Z-Machine style)
    SetPropertyByNumber {
        object: IrId,
        property_num: u8,
        value: IrId,
    },

    /// Test attribute (Z-Machine style) - generates branch instruction
    /// NOTE: Z-Machine test_attr is a branch instruction, not a store instruction
    /// DEPRECATED: Use TestAttributeBranch or TestAttributeValue instead
    /// This instruction has broken codegen and should not be used
    #[deprecated(
        note = "Use TestAttributeValue for value contexts or TestAttributeBranch for branch contexts"
    )]
    TestAttribute {
        target: IrId,
        object: IrId,
        attribute_num: u8,
    },

    /// Direct conditional branch for Z-Machine branch instructions
    /// Used in conditional contexts like: if obj.open { ... }
    /// Generates: test_attr obj, attr -> branch to then_label if true, fall through to else_label if false
    TestAttributeBranch {
        object: IrId,
        attribute_num: u8,
        then_label: IrId, // Branch target if attribute is set
        else_label: IrId, // Fall-through target if attribute is clear
    },

    /// Boolean value extraction from attributes (complex case)
    /// Used in value contexts like: let is_open = obj.open
    /// Generates: test_attr -> branch -> store 1 -> jump end -> store 0 -> end pattern
    TestAttributeValue {
        target: IrId, // Store boolean result (0 or 1)
        object: IrId,
        attribute_num: u8,
    },

    /// Set attribute (Z-Machine style) - generates set_attr/clear_attr
    SetAttribute {
        object: IrId,
        attribute_num: u8,
        value: bool,
    },

    /// Get next property number (for property iteration)
    GetNextProperty {
        target: IrId,
        object: IrId,
        current_property: u8, // 0 for first property
    },

    /// Test if object has a property
    TestProperty {
        target: IrId,
        object: IrId,
        property_num: u8,
    },

    /// Get first child of object (for object tree traversal)
    /// Branch is taken when object has NO child (returns 0)
    GetObjectChild {
        target: IrId,
        object: IrId,
        branch_if_no_child: IrId, // Label to branch to when no child exists
    },

    /// Get next sibling of object (for object tree traversal)
    /// Branch is taken when object has NO sibling (returns 0)
    GetObjectSibling {
        target: IrId,
        object: IrId,
        branch_if_no_sibling: IrId, // Label to branch to when no sibling exists
    },

    /// Get sibling of object without branching (Z-Machine get_sibling instruction)
    /// Returns the sibling object number (0 if no sibling)
    GetObjectSiblingValue {
        target: IrId,
        object: IrId,
    },

    /// Get parent of object (Z-Machine get_parent instruction)
    /// Returns the parent object number (0 if no parent)
    GetObjectParent {
        target: IrId,
        object: IrId,
    },

    /// Insert object into destination (Z-Machine insert_obj instruction)
    /// Sets object's parent to destination, updates object tree structure
    InsertObj {
        object: IrId,
        destination: IrId,
    },

    /// Debug breakpoint (conditional compilation - debug builds only)
    #[cfg(debug_assertions)]
    DebugBreak {
        label: String,
    },

    /// String utility operations
    StringIndexOf {
        target: IrId,
        string: IrId,
        substring: IrId,
    },
    StringSlice {
        target: IrId,
        string: IrId,
        start: IrId,
    },
    StringSubstring {
        target: IrId,
        string: IrId,
        start: IrId,
        end: IrId,
    },
    StringToLowerCase {
        target: IrId,
        string: IrId,
    },
    StringToUpperCase {
        target: IrId,
        string: IrId,
    },
    StringTrim {
        target: IrId,
        string: IrId,
    },
    StringCharAt {
        target: IrId,
        string: IrId,
        index: IrId,
    },
    StringSplit {
        target: IrId,
        string: IrId,
        delimiter: IrId,
    },
    StringReplace {
        target: IrId,
        string: IrId,
        search: IrId,
        replacement: IrId,
    },
    StringStartsWith {
        target: IrId,
        string: IrId,
        prefix: IrId,
    },
    StringEndsWith {
        target: IrId,
        string: IrId,
        suffix: IrId,
    },

    /// Math utility operations
    MathAbs {
        target: IrId,
        value: IrId,
    },
    MathMin {
        target: IrId,
        a: IrId,
        b: IrId,
    },
    MathMax {
        target: IrId,
        a: IrId,
        b: IrId,
    },
    MathRound {
        target: IrId,
        value: IrId,
    },
    MathFloor {
        target: IrId,
        value: IrId,
    },
    MathCeil {
        target: IrId,
        value: IrId,
    },

    /// Type checking operations
    TypeCheck {
        target: IrId,
        value: IrId,
        type_name: String, // "string", "int", "bool", "array", "object"
    },
    TypeOf {
        target: IrId,
        value: IrId,
    },

    /// Print string
    Print {
        value: IrId,
    },

    /// Static array creation - ONLY for compile-time known arrays
    /// Creates a static array with predefined elements [1, 2, 3] or ["a", "b", "c"]
    /// No empty [] arrays - that was the anti-pattern eliminated from Z-Machine games
    CreateArray {
        target: IrId,
        elements: Vec<IrValue>, // Always populated: [1,2,3] or ["a","b","c"]
    },

    /// Array read access - for property arrays and static data
    /// Generates loadw operations following Zork I patterns
    GetArrayElement {
        target: IrId,
        array: IrId,
        index: IrId,
    },

    /// No-operation (used for optimization)
    Nop,
}

/// IR Values - constants and literals
#[derive(Debug, Clone)]
pub enum IrValue {
    Integer(i16),
    Boolean(bool),
    String(String),
    StringRef(IrId),          // Reference to string table entry
    StringAddress(i16), // Packed string address for Z-Machine (result of builtins like exit_get_message)
    Object(String), // Object reference by name - will be resolved to runtime number during codegen
    RuntimeParameter(String), // Grammar parameter like $noun - resolved at runtime from parse buffer
    Null,
}

/// Variable source tracking - tracks where a variable's value originated
/// This is critical for selecting correct iteration strategy in for-loops
#[derive(Debug, Clone, PartialEq)]
pub enum VariableSource {
    /// Variable holds result of obj.contents() call (object tree root)
    /// Contains the container object ID
    ObjectTreeRoot(IrId),

    /// Variable holds scalar value (numbers, booleans, strings, objects)
    /// Not iterable
    Scalar(IrId),
}

/// Binary operations in IR
#[derive(Debug, Clone, PartialEq)]
pub enum IrBinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    // Logical
    And,
    Or,
}

/// Unary operations in IR
#[derive(Debug, Clone, PartialEq)]
pub enum IrUnaryOp {
    Not,
    Minus,
}

impl IrProgram {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            globals: Vec::new(),
            rooms: Vec::new(),
            objects: Vec::new(),
            grammar: Vec::new(),
            init_block: None,
            init_block_locals: Vec::new(),
            string_table: IndexMap::new(),
            property_defaults: IrPropertyDefaults::new(),
            program_mode: ProgramMode::Script, // Default mode, will be overridden
            symbol_ids: IndexMap::new(),
            object_numbers: IndexMap::new(),
            id_registry: IrIdRegistry::new(), // NEW: Initialize ID registry
            property_manager: PropertyManager::new(), // Initialize property manager
            expression_types: IndexMap::new(), // NEW: Initialize expression types for StringAddress system
            system_messages: IndexMap::new(),  // NEW: Initialize system messages catalog
        }
    }

    /// Add a string to the string table and return its ID
    pub fn add_string(&mut self, s: String, id_counter: &mut IrId) -> IrId {
        if let Some(&existing_id) = self.string_table.get(&s) {
            existing_id
        } else {
            let new_id = *id_counter;
            *id_counter += 1;
            self.string_table.insert(s, new_id);
            new_id
        }
    }
}

impl IrBlock {
    pub fn new(id: IrId) -> Self {
        Self {
            id,
            instructions: Vec::new(),
        }
    }

    pub fn add_instruction(&mut self, instruction: IrInstruction) {
        self.instructions.push(instruction);
    }
}

impl Default for IrProgram {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert AST binary operators to IR binary operators
impl From<crate::grue_compiler::ast::BinaryOp> for IrBinaryOp {
    fn from(op: crate::grue_compiler::ast::BinaryOp) -> Self {
        use crate::grue_compiler::ast::BinaryOp as AstOp;
        match op {
            AstOp::Add => IrBinaryOp::Add,
            AstOp::Subtract => IrBinaryOp::Subtract,
            AstOp::Multiply => IrBinaryOp::Multiply,
            AstOp::Divide => IrBinaryOp::Divide,
            AstOp::Modulo => IrBinaryOp::Modulo,
            AstOp::Equal => IrBinaryOp::Equal,
            AstOp::NotEqual => IrBinaryOp::NotEqual,
            AstOp::Less => IrBinaryOp::Less,
            AstOp::LessEqual => IrBinaryOp::LessEqual,
            AstOp::Greater => IrBinaryOp::Greater,
            AstOp::GreaterEqual => IrBinaryOp::GreaterEqual,
            AstOp::And => IrBinaryOp::And,
            AstOp::Or => IrBinaryOp::Or,
        }
    }
}

/// Convert AST unary operators to IR unary operators
impl From<crate::grue_compiler::ast::UnaryOp> for IrUnaryOp {
    fn from(op: crate::grue_compiler::ast::UnaryOp) -> Self {
        use crate::grue_compiler::ast::UnaryOp as AstOp;
        match op {
            AstOp::Not => IrUnaryOp::Not,
            AstOp::Minus => IrUnaryOp::Minus,
        }
    }
}

// IR Generator module
#[path = "ir_generator.rs"]
mod ir_generator;
pub use ir_generator::IrGenerator;

#[cfg(test)]
#[path = "ir_tests.rs"]
mod tests;

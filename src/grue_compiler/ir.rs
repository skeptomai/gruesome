// Intermediate Representation for Grue Language
//
// The IR is designed to be a lower-level representation that's closer to Z-Machine
// instructions while still maintaining some high-level constructs for optimization.

use crate::grue_compiler::ast::{Program, ProgramMode, Type};
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::object_system::ComprehensiveObject;
use indexmap::{IndexMap, IndexSet};
use std::collections::HashMap;

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
    pub defaults: HashMap<u8, u16>, // Property number -> default value
}

impl IrPropertyDefaults {
    pub fn new() -> Self {
        Self {
            defaults: HashMap::new(),
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
    pub properties: HashMap<u8, IrPropertyValue>, // Property number -> value
}

impl IrProperties {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
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
    // Common attributes used in Zork and other games
    Invisible = 0,    // Object is not listed in room descriptions
    Container = 1,    // Object can contain other objects
    Openable = 2,     // Object can be opened/closed
    Open = 3,         // Object is currently open
    Takeable = 4,     // Object can be picked up
    Moved = 5,        // Object has been moved from initial location
    Worn = 6,         // Object is being worn
    LightSource = 7,  // Object provides light
    Visited = 8,      // Room has been visited
    Locked = 9,       // Object is locked
    Edible = 10,      // Object can be eaten
    Treasure = 11,    // Object is a treasure for scoring
    Special = 12,     // Object has special behavior
    Transparent = 13, // Can see through object to contents
    On = 14,          // Object is switched on (for light sources, etc.)
    Workflag = 15,    // Temporary flag for game logic
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

    /// Create array
    CreateArray {
        target: IrId,
        size: IrValue,
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

    /// Array access
    GetArrayElement {
        target: IrId,
        array: IrId,
        index: IrId,
    },

    /// Array assignment
    SetArrayElement {
        array: IrId,
        index: IrId,
        value: IrId,
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

    /// Array operations
    ArrayAdd {
        array: IrId,
        value: IrId,
    },
    ArrayRemove {
        target: IrId, // Result storage
        array: IrId,
        index: IrId,
    },

    /// Debug breakpoint (conditional compilation - debug builds only)
    #[cfg(debug_assertions)]
    DebugBreak {
        label: String,
    },
    ArrayLength {
        target: IrId,
        array: IrId,
    },
    ArrayEmpty {
        target: IrId,
        array: IrId,
    },
    ArrayContains {
        target: IrId,
        array: IrId,
        value: IrId,
    },

    /// Advanced array operations
    ArrayFilter {
        target: IrId,
        array: IrId,
        predicate: IrId, // Function to call for each element
    },
    ArrayMap {
        target: IrId,
        array: IrId,
        transform: IrId, // Function to call for each element
    },
    ArrayForEach {
        array: IrId,
        callback: IrId, // Function to call for each element
    },
    ArrayFind {
        target: IrId,
        array: IrId,
        predicate: IrId, // Function to call for each element
    },
    ArrayIndexOf {
        target: IrId,
        array: IrId,
        value: IrId,
    },
    ArrayJoin {
        target: IrId,
        array: IrId,
        separator: IrId,
    },
    ArrayReverse {
        target: IrId,
        array: IrId,
    },
    ArraySort {
        target: IrId,
        array: IrId,
        comparator: Option<IrId>, // Optional comparison function
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

    /// No-operation (used for optimization)
    Nop,
}

/// IR Values - constants and literals
#[derive(Debug, Clone)]
pub enum IrValue {
    Integer(i16),
    Boolean(bool),
    String(String),
    StringRef(IrId), // Reference to string table entry
    Null,
}

/// Variable source tracking - tracks where a variable's value originated
/// This is critical for selecting correct iteration strategy in for-loops
#[derive(Debug, Clone, PartialEq)]
pub enum VariableSource {
    /// Variable holds result of obj.contents() call (object tree root)
    /// Contains the container object ID
    ObjectTreeRoot(IrId),

    /// Variable holds array (from literal or CreateArray)
    /// Contains the array IR ID
    Array(IrId),

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

/// IR Generator - converts AST to IR
pub struct IrGenerator {
    id_counter: IrId,
    symbol_ids: IndexMap<String, IrId>, // Symbol name -> IR ID mapping
    current_locals: Vec<IrLocal>,       // Track local variables in current function
    next_local_slot: u8,                // Next available local variable slot
    builtin_functions: IndexMap<IrId, String>, // Function ID -> Function name for builtins
    object_numbers: IndexMap<String, u16>, // Object name -> Object number mapping
    object_counter: u16,                // Next available object number (starts at 2, player is 1)
    property_manager: PropertyManager,  // Manages property numbering and inheritance
    id_registry: IrIdRegistry,          // NEW: Track all IR IDs for debugging and mapping
    variable_sources: IndexMap<IrId, VariableSource>, // Track variable origins for iteration strategy
    /// Mapping of room names to objects contained within them
    /// Used for automatic object placement during init block generation
    room_objects: IndexMap<String, Vec<RoomObjectInfo>>,
}

impl Default for IrGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IrGenerator {
    pub fn new() -> Self {
        let mut object_numbers = IndexMap::new();
        // Player is always object #1
        object_numbers.insert("player".to_string(), 1);

        IrGenerator {
            id_counter: 1, // Start from 1, 0 is reserved
            symbol_ids: IndexMap::new(),
            current_locals: Vec::new(),
            next_local_slot: 1, // Slot 0 reserved for return value
            builtin_functions: IndexMap::new(),
            object_numbers,
            object_counter: 2, // Start at 2, player is object #1
            property_manager: PropertyManager::new(),
            id_registry: IrIdRegistry::new(), // NEW: Initialize ID registry
            variable_sources: IndexMap::new(), // NEW: Initialize variable source tracking
            room_objects: IndexMap::new(),    // NEW: Initialize room object mapping
        }
    }

    /// Check if a function name is a known builtin function
    fn is_builtin_function(&self, name: &str) -> bool {
        #[cfg(debug_assertions)]
        {
            matches!(
                name,
                "print"
                    | "print_ret"
                    | "new_line"
                    | "move"
                    | "get_location"
                    | "get_child"
                    | "get_sibling"
                    | "get_prop"
                    | "test_attr"
                    | "set_attr"
                    | "clear_attr"
                    | "to_string"
                    | "random"
                    // String utility functions
                    | "indexOf"
                    | "slice"
                    | "substring"
                    | "toLowerCase"
                    | "toUpperCase"
                    | "trim"
                    | "charAt"
                    | "split"
                    | "replace"
                    | "startsWith"
                    | "endsWith"
                    // Math utility functions
                    | "abs"
                    | "min"
                    | "max"
                    | "round"
                    | "floor"
                    | "ceil"
                    // Type checking functions
                    | "is_string"
                    | "is_int"
                    | "is_bool"
                    | "is_array"
                    | "is_object"
                    | "typeof"
                    // Game control
                    | "quit"
                    // Debug breakpoints (debug builds only)
                    | "debug_break"
            )
        }
        #[cfg(not(debug_assertions))]
        {
            matches!(
                name,
                "print"
                    | "print_ret"
                    | "new_line"
                    | "move"
                    | "get_location"
                    | "get_child"
                    | "get_sibling"
                    | "get_prop"
                    | "test_attr"
                    | "set_attr"
                    | "clear_attr"
                    | "to_string"
                    | "random"
                    // String utility functions
                    | "indexOf"
                    | "slice"
                    | "substring"
                    | "toLowerCase"
                    | "toUpperCase"
                    | "trim"
                    | "charAt"
                    | "split"
                    | "replace"
                    | "startsWith"
                    | "endsWith"
                    // Math utility functions
                    | "abs"
                    | "min"
                    | "max"
                    | "round"
                    | "floor"
                    | "ceil"
                    // Type checking functions
                    | "is_string"
                    | "is_int"
                    | "is_bool"
                    | "is_array"
                    | "is_object"
                    | "typeof"
                    // Game control
                    | "quit"
            )
        }
    }

    pub fn generate(&mut self, ast: Program) -> Result<IrProgram, CompilerError> {
        log::debug!(
            "IR GENERATOR: Starting IR generation for {} items",
            ast.items.len()
        );
        let mut ir_program = IrProgram::new();

        // Detect program mode from AST
        let program_mode = ast.detect_program_mode();
        ir_program.program_mode = program_mode.clone();
        log::debug!("Detected program mode: {:?}", program_mode);

        // TWO-PASS APPROACH: First pass registers all function definitions to populate symbol table
        // This ensures function calls can resolve to actual definitions, not placeholders
        for item in ast.items.iter() {
            if let crate::grue_compiler::ast::Item::Function(func) = item {
                // Register function name in symbol table (but don't generate IR body yet)
                let func_id = self.next_id();
                self.symbol_ids.insert(func.name.clone(), func_id);
                log::debug!(
                    "PASS 1: Registered function '{}' with ID {}",
                    func.name,
                    func_id
                );
            }
        }

        // SECOND PASS: Generate IR for all items (functions will now use registered IDs)
        for item in ast.items.iter() {
            self.generate_item(item.clone(), &mut ir_program)?;
        }

        // Add synthetic player object to IR
        // The player is object #1 and needs to be in the IR like all other objects
        self.add_player_object(&mut ir_program)?;

        // Copy symbol mappings from generator to IR program for use in codegen
        ir_program.symbol_ids = self.symbol_ids.clone();
        ir_program.object_numbers = self.object_numbers.clone();
        ir_program.id_registry = self.id_registry.clone(); // NEW: Transfer ID registry
        ir_program.property_manager = self.property_manager.clone(); // Transfer property manager with consistent mappings

        Ok(ir_program)
    }

    /// Get builtin functions discovered during IR generation
    pub fn get_builtin_functions(&self) -> &IndexMap<IrId, String> {
        &self.builtin_functions
    }

    pub fn get_object_numbers(&self) -> &IndexMap<String, u16> {
        &self.object_numbers
    }

    /// Check if a property name corresponds to a standard Z-Machine property
    fn get_standard_property(&self, property_name: &str) -> Option<StandardProperty> {
        match property_name {
            "short_name" | "name" => Some(StandardProperty::ShortName),
            "long_name" => Some(StandardProperty::LongName),
            "desc" | "description" => Some(StandardProperty::Description),
            "initial" => Some(StandardProperty::Initial),
            "before" => Some(StandardProperty::Before),
            "after" => Some(StandardProperty::After),
            "life" => Some(StandardProperty::Life),
            "capacity" => Some(StandardProperty::Capacity),
            "value" => Some(StandardProperty::Value),
            "size" => Some(StandardProperty::Size),
            "article" => Some(StandardProperty::Article),
            "adjective" => Some(StandardProperty::Adjective),
            _ => None,
        }
    }

    fn next_id(&mut self) -> IrId {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }

    /// Centralized IR instruction emission with automatic ID tracking
    /// This ensures all IR IDs are properly registered for codegen mapping
    fn emit_ir_instruction(&mut self, block: &mut IrBlock, instruction: IrInstruction) -> IrId {
        let target_id = match &instruction {
            // Extract target ID from instructions that create new values
            IrInstruction::LoadImmediate { target, .. } => Some(*target),
            IrInstruction::LoadVar { target, .. } => Some(*target),
            IrInstruction::StoreVar { var_id, .. } => Some(*var_id),
            IrInstruction::BinaryOp { target, .. } => Some(*target),
            IrInstruction::UnaryOp { target, .. } => Some(*target),
            IrInstruction::Call { target, .. } => *target,
            IrInstruction::GetProperty { target, .. } => Some(*target),
            IrInstruction::GetPropertyByNumber { target, .. } => Some(*target),
            IrInstruction::SetProperty { .. } => None,
            IrInstruction::SetPropertyByNumber { .. } => None,
            IrInstruction::CreateArray { target, .. } => Some(*target),
            IrInstruction::Jump { .. } => None,
            IrInstruction::Label { .. } => None,
            IrInstruction::Return { .. } => None,
            _ => None,
        };

        // Track this IR ID and its type for debugging and mapping
        if let Some(tid) = target_id {
            let instruction_type = match &instruction {
                IrInstruction::LoadImmediate { .. } => "LoadImmediate",
                IrInstruction::LoadVar { .. } => "LoadVar",
                IrInstruction::StoreVar { .. } => "StoreVar",
                IrInstruction::BinaryOp { .. } => "BinaryOp",
                IrInstruction::UnaryOp { .. } => "UnaryOp",
                IrInstruction::Call { .. } => "Call",
                IrInstruction::GetProperty { .. } => "GetProperty",
                IrInstruction::GetPropertyByNumber { .. } => "GetPropertyByNumber",
                IrInstruction::CreateArray { .. } => "CreateArray",
                _ => "Other",
            };

            // Register this IR ID in the centralized registry
            self.id_registry
                .register_expression_id(tid, instruction_type);

            log::debug!(
                "IR EMISSION: ID {} <- {} instruction",
                tid,
                instruction_type
            );

            // Debug: Track problematic ID range
            if (80..=100).contains(&tid) {
                log::warn!(
                    "TRACKING PROBLEMATIC ID {}: {} instruction",
                    tid,
                    instruction_type
                );
            }
        }

        block.add_instruction(instruction);
        target_id.unwrap_or(0) // Return the target ID or 0 if no target
    }

    fn generate_item(
        &mut self,
        item: crate::grue_compiler::ast::Item,
        ir_program: &mut IrProgram,
    ) -> Result<(), CompilerError> {
        use crate::grue_compiler::ast::Item;

        match item {
            Item::Function(func) => {
                let ir_func = self.generate_function(func)?;
                ir_program.functions.push(ir_func);
            }
            Item::World(world) => {
                self.generate_world(world, ir_program)?;
            }
            Item::Grammar(grammar) => {
                let ir_grammar = self.generate_grammar(grammar)?;
                ir_program.grammar.extend(ir_grammar);
            }
            Item::Init(init) => {
                let mut ir_block = self.generate_block(init.body)?;

                // Phase 1c: Inject object placement instructions after user's init code
                self.generate_object_placement_instructions(&mut ir_block)?;

                ir_program.init_block = Some(ir_block);

                // Save local variables declared in init block (e.g., let statements)
                // These need to be tracked separately since init blocks are IrBlock not IrFunction
                ir_program.init_block_locals = self.current_locals.clone();
                self.current_locals.clear(); // Clear for next function/block
                self.next_local_slot = 1; // Reset slot counter
            }
            Item::Mode(_mode) => {
                // Mode declarations are handled during program mode detection in generate()
                // No IR generation needed for the mode declaration itself
            }
        }

        Ok(())
    }

    fn generate_function(
        &mut self,
        func: crate::grue_compiler::ast::FunctionDecl,
    ) -> Result<IrFunction, CompilerError> {
        // Function should already be registered in symbol table from first pass
        let func_id = if let Some(&existing_id) = self.symbol_ids.get(&func.name) {
            existing_id
        } else {
            return Err(CompilerError::SemanticError(format!(
                "Function '{}' not found in symbol table. This indicates a bug in the two-pass system.",
                func.name
            ), 0));
        };

        // SCOPE MANAGEMENT: Save the current global symbol table before processing function
        let saved_symbol_ids = self.symbol_ids.clone();

        // Reset local variable state for this function
        self.current_locals.clear();
        self.next_local_slot = 1; // Slot 0 reserved for return value

        let mut parameters = Vec::new();
        log::debug!(
            "🔧 IR_DEBUG: Function '{}' has {} parameters in AST",
            func.name,
            func.parameters.len()
        );

        // Add parameters as local variables
        for (i, param) in func.parameters.iter().enumerate() {
            log::debug!(
                "🔧 IR_DEBUG: Processing parameter [{}/{}] '{}' for function '{}'",
                i + 1,
                func.parameters.len(),
                param.name,
                func.name
            );

            let param_id = self.next_id();
            let ir_param = IrParameter {
                name: param.name.clone(),
                param_type: param.param_type.clone(),
                slot: self.next_local_slot,
                ir_id: param_id,
            };

            // Add parameters to FUNCTION-SCOPED symbol table (not global)
            self.symbol_ids.insert(param.name.clone(), param_id);
            log::debug!(
                "Function '{}': Added parameter '{}' with IR ID {} to function scope",
                func.name,
                param.name,
                param_id
            );

            // Add parameter as local variable
            let local_param = IrLocal {
                ir_id: param_id,
                name: param.name.clone(),
                var_type: param.param_type.clone(),
                slot: self.next_local_slot,
                mutable: true, // Parameters are typically mutable
            };
            self.current_locals.push(local_param);

            parameters.push(ir_param);
            log::debug!(
                "🔧 IR_DEBUG: Added parameter '{}' (IR ID {}) to parameters Vec for function '{}'",
                param.name,
                param_id,
                func.name
            );
            self.next_local_slot += 1;
        }

        // Generate function body with function-scoped parameters
        let body = self.generate_block(func.body)?;
        let local_vars = self.current_locals.clone();

        // SCOPE MANAGEMENT: Restore the global symbol table after processing function
        self.symbol_ids = saved_symbol_ids;

        log::debug!(
            "🔧 IR_DEBUG: Creating IrFunction '{}' with {} parameters",
            func.name,
            parameters.len()
        );
        for (i, param) in parameters.iter().enumerate() {
            log::debug!(
                "🔧 IR_DEBUG: Final parameter [{}/{}]: name='{}', ir_id={}, slot={}",
                i + 1,
                parameters.len(),
                param.name,
                param.ir_id,
                param.slot
            );
        }

        Ok(IrFunction {
            id: func_id,
            name: func.name.clone(),
            parameters,
            return_type: func.return_type,
            body,
            local_vars,
        })
    }

    fn generate_world(
        &mut self,
        world: crate::grue_compiler::ast::WorldDecl,
        ir_program: &mut IrProgram,
    ) -> Result<(), CompilerError> {
        // First pass: register all rooms and objects for symbol resolution
        for room in &world.rooms {
            let room_id = self.next_id();
            self.symbol_ids.insert(room.identifier.clone(), room_id);
            // Register in the centralized registry as a named symbol
            self.id_registry
                .register_id(room_id, "room", "generate_world", false);

            let object_number = self.object_counter;
            self.object_counter += 1;
            self.object_numbers
                .insert(room.identifier.clone(), object_number);

            // Register all objects in the room
            for obj in &room.objects {
                self.register_object_and_nested(obj)?;
            }
        }

        // Second pass: generate actual IR objects and rooms
        for room in world.rooms {
            let ir_room = self.generate_room(room.clone())?;
            let room_id = ir_room.id; // Save the room ID before moving ir_room
            ir_program.rooms.push(ir_room);

            // Generate IR objects for this room
            for obj in room.objects {
                let ir_objects = self.generate_object(obj, Some(room_id))?;
                ir_program.objects.extend(ir_objects);
            }
        }

        // Set up property defaults for common properties
        self.setup_property_defaults(ir_program);

        Ok(())
    }

    /// Set up default values for standard Z-Machine properties
    fn setup_property_defaults(&self, ir_program: &mut IrProgram) {
        // Set sensible defaults for common properties
        if let Some(short_name_num) = self
            .property_manager
            .get_standard_property_number(StandardProperty::ShortName)
        {
            ir_program.property_defaults.set_default(short_name_num, 0); // Empty string by default
        }

        if let Some(capacity_num) = self
            .property_manager
            .get_standard_property_number(StandardProperty::Capacity)
        {
            ir_program.property_defaults.set_default(capacity_num, 100); // Default container capacity
        }

        if let Some(value_num) = self
            .property_manager
            .get_standard_property_number(StandardProperty::Value)
        {
            ir_program.property_defaults.set_default(value_num, 0); // Default object value
        }

        if let Some(size_num) = self
            .property_manager
            .get_standard_property_number(StandardProperty::Size)
        {
            ir_program.property_defaults.set_default(size_num, 5); // Default object size
        }
    }

    fn generate_object(
        &mut self,
        obj: crate::grue_compiler::ast::ObjectDecl,
        parent_id: Option<IrId>,
    ) -> Result<Vec<IrObject>, CompilerError> {
        let mut result = Vec::new();

        // Get the object ID that was registered earlier
        let obj_id = *self
            .symbol_ids
            .get(&obj.identifier)
            .ok_or_else(|| CompilerError::UndefinedSymbol(obj.identifier.clone(), 0))?;

        // Convert named attributes to Z-Machine attributes
        let mut attributes = IrAttributes::new();
        for attr_name in &obj.attributes {
            match attr_name.as_str() {
                "openable" => attributes.set(StandardAttribute::Openable as u8, true),
                "container" => attributes.set(StandardAttribute::Container as u8, true),
                "takeable" => attributes.set(StandardAttribute::Takeable as u8, true),
                "light_source" => attributes.set(StandardAttribute::LightSource as u8, true),
                "treasure" => attributes.set(StandardAttribute::Treasure as u8, true),
                "edible" => attributes.set(StandardAttribute::Edible as u8, true),
                "worn" => attributes.set(StandardAttribute::Worn as u8, true),
                "locked" => attributes.set(StandardAttribute::Locked as u8, true),
                "transparent" => attributes.set(StandardAttribute::Transparent as u8, true),
                _ => {
                    log::warn!(
                        "Unknown attribute '{}' on object '{}'",
                        attr_name,
                        obj.identifier
                    );
                }
            }
        }

        // Set attributes based on properties (for backward compatibility)
        for (prop_name, prop_value) in &obj.properties {
            match prop_name.as_str() {
                "openable" => {
                    if let crate::grue_compiler::ast::PropertyValue::Boolean(true) = prop_value {
                        attributes.set(StandardAttribute::Openable as u8, true);
                    }
                }
                "open" => {
                    if let crate::grue_compiler::ast::PropertyValue::Boolean(true) = prop_value {
                        attributes.set(StandardAttribute::Open as u8, true);
                    }
                }
                "container" => {
                    if let crate::grue_compiler::ast::PropertyValue::Boolean(true) = prop_value {
                        attributes.set(StandardAttribute::Container as u8, true);
                    }
                }
                _ => {} // Other properties handled below
            }
        }

        // CRITICAL FIX (Oct 28, 2025): Object name property must use first name from names array
        // Previously used obj.identifier which caused "mailbox" instead of "small mailbox"
        // Bug: obj.name accessed short_name property which was set incorrectly
        // Fix: Use first name from names array, falling back to identifier if names is empty
        let short_name = obj
            .names
            .first()
            .cloned()
            .unwrap_or_else(|| obj.identifier.clone());

        // Convert properties to Z-Machine properties
        let mut properties = IrProperties::new();

        // Set standard properties using computed short_name (not obj.identifier!)
        properties.set_string(StandardProperty::ShortName as u8, short_name.clone());
        properties.set_string(StandardProperty::LongName as u8, obj.description.clone());

        // Convert AST properties to Z-Machine properties using property manager
        for (prop_name, prop_value) in &obj.properties {
            let prop_num = self.property_manager.get_property_number(prop_name);
            match prop_value {
                crate::grue_compiler::ast::PropertyValue::Boolean(val) => {
                    properties.set_byte(prop_num, if *val { 1 } else { 0 });
                }
                crate::grue_compiler::ast::PropertyValue::Integer(val) => {
                    if *val >= 0 {
                        properties.set_word(prop_num, *val as u16);
                    }
                }
                crate::grue_compiler::ast::PropertyValue::String(val) => {
                    properties.set_string(prop_num, val.clone());
                }
                crate::grue_compiler::ast::PropertyValue::Byte(val) => {
                    properties.set_byte(prop_num, *val);
                }
                crate::grue_compiler::ast::PropertyValue::Bytes(val) => {
                    properties.set_bytes(prop_num, val.clone());
                }
                crate::grue_compiler::ast::PropertyValue::Object(obj_name) => {
                    // Convert object reference to object number when available
                    if let Some(&obj_num) = self.object_numbers.get(obj_name) {
                        properties.set_word(prop_num, obj_num);
                    }
                }
                crate::grue_compiler::ast::PropertyValue::Room(room_name) => {
                    // Convert room reference to room number when available
                    if let Some(&room_num) = self.object_numbers.get(room_name) {
                        properties.set_word(prop_num, room_num);
                    }
                }
            }
        }

        // Convert numbered properties
        for (prop_num, prop_value) in &obj.numbered_properties {
            match prop_value {
                crate::grue_compiler::ast::PropertyValue::Byte(val) => {
                    properties.set_byte(*prop_num, *val);
                }
                crate::grue_compiler::ast::PropertyValue::Integer(val) => {
                    if *val >= 0 {
                        properties.set_word(*prop_num, *val as u16);
                    }
                }
                crate::grue_compiler::ast::PropertyValue::String(val) => {
                    properties.set_string(*prop_num, val.clone());
                }
                crate::grue_compiler::ast::PropertyValue::Bytes(val) => {
                    properties.set_bytes(*prop_num, val.clone());
                }
                crate::grue_compiler::ast::PropertyValue::Object(obj_name) => {
                    // Convert object reference to object number when available
                    if let Some(&obj_num) = self.object_numbers.get(obj_name) {
                        properties.set_word(*prop_num, obj_num);
                    }
                }
                crate::grue_compiler::ast::PropertyValue::Room(room_name) => {
                    // Convert room reference to room number when available
                    if let Some(&room_num) = self.object_numbers.get(room_name) {
                        properties.set_word(*prop_num, room_num);
                    }
                }
                crate::grue_compiler::ast::PropertyValue::Boolean(_) => {
                    // Already handled above, but included for exhaustiveness
                }
            }
        }

        // Process contains relationship - convert to parent/child relationships
        let mut child_objects = Vec::new();
        for contained_obj in obj.contains {
            let child_ir_objects = self.generate_object(contained_obj, Some(obj_id))?;
            for child in &child_ir_objects {
                child_objects.push(child.id);
            }
            result.extend(child_ir_objects);
        }

        // Build the sibling chain for children
        let first_child = child_objects.first().copied();
        for i in 0..child_objects.len() {
            let next_sibling = if i + 1 < child_objects.len() {
                Some(child_objects[i + 1])
            } else {
                None
            };

            // Find the child in result and update its sibling
            if let Some(child) = result.iter_mut().find(|obj| obj.id == child_objects[i]) {
                child.sibling = next_sibling;
            }
        }

        let ir_object = IrObject {
            id: obj_id,
            name: obj.identifier,
            short_name,
            names: obj.names,
            description: obj.description,
            attributes,
            properties,
            parent: parent_id,
            sibling: None, // Will be set when building sibling chains
            child: first_child,
            comprehensive_object: None, // Will be set when enhanced object system is integrated
        };

        result.insert(0, ir_object);
        Ok(result)
    }

    fn register_object_and_nested(
        &mut self,
        obj: &crate::grue_compiler::ast::ObjectDecl,
    ) -> Result<(), CompilerError> {
        // Register the object itself
        let obj_id = self.next_id();
        self.symbol_ids.insert(obj.identifier.clone(), obj_id);

        // Assign object number (check if already assigned to avoid duplicates)
        if !self.object_numbers.contains_key(&obj.identifier) {
            let object_number = self.object_counter;
            self.object_counter += 1;
            self.object_numbers
                .insert(obj.identifier.clone(), object_number);
            log::debug!(
                "Assigned NEW object number {} to object '{}'",
                object_number,
                obj.identifier
            );
        } else {
            log::debug!(
                "Object '{}' already has object number {}",
                obj.identifier,
                self.object_numbers[&obj.identifier]
            );
        }

        log::debug!(
            "Registered object '{}' with ID {} and object number {}",
            obj.identifier,
            obj_id,
            self.object_numbers[&obj.identifier]
        );

        // Process nested objects recursively
        for nested_obj in &obj.contains {
            self.register_object_and_nested(nested_obj)?;
        }

        Ok(())
    }

    /// Extract object hierarchy from AST ObjectDecl for room object mapping
    /// Converts ObjectDecl and its nested objects into RoomObjectInfo structure
    fn extract_object_hierarchy(
        &self,
        obj: &crate::grue_compiler::ast::ObjectDecl,
    ) -> RoomObjectInfo {
        // Extract nested objects recursively
        let nested_objects: Vec<RoomObjectInfo> = obj
            .contains
            .iter()
            .map(|nested_obj| self.extract_object_hierarchy(nested_obj))
            .collect();

        RoomObjectInfo {
            name: obj.identifier.clone(),
            nested_objects,
        }
    }

    /// Generate InsertObj instructions from room_objects mapping for init block
    /// Converts room object hierarchies to InsertObj instructions to establish object tree
    fn generate_object_placement_instructions(
        &self,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "Phase 1c: Generating object placement instructions for {} rooms",
            self.room_objects.len()
        );

        for (room_name, objects) in &self.room_objects {
            // Look up room IR ID from symbol table
            let room_ir_id = *self.symbol_ids.get(room_name).ok_or_else(|| {
                CompilerError::CodeGenError(format!(
                    "Room '{}' not found in symbol table during object placement",
                    room_name
                ))
            })?;

            log::debug!(
                "Phase 1c: Placing {} objects in room '{}' (IR ID {})",
                objects.len(),
                room_name,
                room_ir_id
            );

            // Generate placement instructions for each object in this room
            for object_info in objects {
                self.generate_placement_for_object(object_info, room_ir_id, block)?;
            }
        }

        Ok(())
    }

    /// Generate InsertObj instructions for a single object and its nested objects
    /// Recursively handles object containment hierarchy
    fn generate_placement_for_object(
        &self,
        object_info: &RoomObjectInfo,
        container_ir_id: u32,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        // Look up object IR ID from symbol table
        let object_ir_id = *self.symbol_ids.get(&object_info.name).ok_or_else(|| {
            CompilerError::CodeGenError(format!(
                "Object '{}' not found in symbol table during placement",
                object_info.name
            ))
        })?;

        // Generate InsertObj instruction to place this object in its container
        block.instructions.push(IrInstruction::InsertObj {
            object: object_ir_id,
            destination: container_ir_id,
        });

        log::debug!(
            "Phase 1c: Generated InsertObj for '{}' (IR {}) into container (IR {})",
            object_info.name,
            object_ir_id,
            container_ir_id
        );

        // Recursively handle nested objects (they go inside this object)
        for nested_object in &object_info.nested_objects {
            self.generate_placement_for_object(nested_object, object_ir_id, block)?;
        }

        Ok(())
    }

    /// Add synthetic player object to IR program
    /// The player is always object #1 and has standard properties
    fn add_player_object(&mut self, ir_program: &mut IrProgram) -> Result<(), CompilerError> {
        // Create player object with ID 9999 (high ID to avoid conflicts)
        let player_id = 9999u32;

        // Register player in symbol table
        self.symbol_ids.insert("player".to_string(), player_id);

        // Player is always object #1 in Z-Machine
        // (Object numbers were incremented for rooms/objects, but player is inserted first during codegen)

        // Create player properties
        let mut player_properties = IrProperties::new();

        // Get property numbers from property manager
        let location_prop = self.property_manager.get_property_number("location");
        let desc_prop = self
            .property_manager
            .get_property_number_by_name("description")
            .or_else(|| self.property_manager.get_property_number_by_name("desc"))
            .unwrap_or(7); // Default to property 7 if not found

        // Set initial player location to first room (will be room object #2 during codegen)
        let initial_location = if !ir_program.rooms.is_empty() { 2 } else { 0 };
        player_properties.set_word(location_prop, initial_location);

        // Set player description
        player_properties.set_string(desc_prop, "yourself".to_string());

        // Add quit_pending property for quit confirmation flow
        let quit_pending_prop = self.property_manager.get_property_number("quit_pending");
        player_properties.set_word(quit_pending_prop, 0); // Initially false

        // Create player object
        // BUG FIX (Oct 11, 2025): Set player's initial parent to match location property
        // Since player.location now reads from object tree (get_parent), not property,
        // we must initialize the tree parent to match the location property value
        let initial_parent = if !ir_program.rooms.is_empty() {
            // Player starts in first room, which will be object #2 (player is #1)
            // Store IR ID of first room as parent
            Some(ir_program.rooms[0].id)
        } else {
            None
        };

        let player_object = IrObject {
            id: player_id,
            name: "player".to_string(),
            short_name: "yourself".to_string(),
            description: String::new(), // Description is in properties
            names: vec!["yourself".to_string()],
            attributes: IrAttributes::new(),
            properties: player_properties,
            parent: initial_parent, // Start as child of first room
            sibling: None,
            child: None, // Player can contain objects (inventory)
            comprehensive_object: None,
        };

        // Add player as first object (it will become object #1 during codegen)
        ir_program.objects.insert(0, player_object);

        log::debug!(
            "Added synthetic player object with ID {} (will be object #1)",
            player_id
        );

        Ok(())
    }

    fn generate_room(
        &mut self,
        room: crate::grue_compiler::ast::RoomDecl,
    ) -> Result<IrRoom, CompilerError> {
        // Room ID should already be pre-registered during first pass
        let room_id = *self.symbol_ids.get(&room.identifier).unwrap_or_else(|| {
            panic!(
                "Room '{}' should have been pre-registered in first pass",
                room.identifier
            )
        });

        // Object numbers should already be assigned during registration pass
        // Don't reassign if already exists (avoids duplicate assignment bug)
        if let Some(&existing_number) = self.object_numbers.get(&room.identifier) {
            log::debug!(
                "IR generate_room: Room '{}' already has object number {} from registration pass",
                room.identifier,
                existing_number
            );
        } else {
            // Fallback: assign object number if not already assigned
            let object_number = self.object_counter;
            self.object_counter += 1;
            log::debug!(
                "IR generate_room: Assigning object number {} to room '{}' (fallback)",
                object_number,
                room.identifier
            );
            self.object_numbers
                .insert(room.identifier.clone(), object_number);
        }

        let mut exits = IndexMap::new();
        log::debug!(
            "IR generate_room: Processing {} exits for room '{}'",
            room.exits.len(),
            room.identifier
        );
        for (direction, target) in room.exits {
            log::debug!("IR generate_room: Exit '{}' -> {:?}", direction, target);
            let ir_target = match target {
                crate::grue_compiler::ast::ExitTarget::Room(room_name) => {
                    // Look up target room IR ID from symbol table
                    let target_room_id = *self.symbol_ids.get(&room_name).unwrap_or(&0);
                    if target_room_id == 0 {
                        return Err(CompilerError::CodeGenError(format!(
                            "Exit from room '{}' references undefined room '{}'",
                            room.identifier, room_name
                        )));
                    }
                    IrExitTarget::Room(target_room_id)
                }
                crate::grue_compiler::ast::ExitTarget::Blocked(message) => {
                    IrExitTarget::Blocked(message)
                }
            };
            exits.insert(direction, ir_target);
        }

        // Process room objects FIRST - add them to symbol_ids for identifier resolution
        // This must happen before processing handlers that might reference these objects
        log::debug!(
            "Processing {} objects for room '{}'",
            room.objects.len(),
            room.identifier
        );

        // Phase 1b: Record object hierarchy in room_objects mapping
        let mut room_object_infos = Vec::new();
        for obj in &room.objects {
            self.register_object_and_nested(obj)?;

            // Extract object hierarchy and add to room mapping
            let object_info = self.extract_object_hierarchy(obj);
            room_object_infos.push(object_info);
        }

        // Store the complete object hierarchy for this room
        if !room_object_infos.is_empty() {
            self.room_objects
                .insert(room.identifier.clone(), room_object_infos);
            log::debug!(
                "Phase 1b: Recorded {} object hierarchies for room '{}'",
                self.room_objects[&room.identifier].len(),
                room.identifier
            );
        }

        // Now process handlers - objects are available for reference
        let on_enter = if let Some(block) = room.on_enter {
            Some(self.generate_block(block)?)
        } else {
            None
        };

        let on_exit = if let Some(block) = room.on_exit {
            Some(self.generate_block(block)?)
        } else {
            None
        };

        let on_look = if let Some(block) = room.on_look {
            Some(self.generate_block(block)?)
        } else {
            None
        };

        Ok(IrRoom {
            id: room_id,
            name: room.identifier,
            display_name: room.display_name,
            description: room.description,
            exits,
            on_enter,
            on_exit,
            on_look,
        })
    }

    fn generate_grammar(
        &mut self,
        grammar: crate::grue_compiler::ast::GrammarDecl,
    ) -> Result<Vec<IrGrammar>, CompilerError> {
        let mut ir_grammar = Vec::new();

        for verb in grammar.verbs {
            let mut patterns = Vec::new();

            for pattern in verb.patterns {
                let ir_pattern_elements: Vec<IrPatternElement> = pattern
                    .pattern
                    .into_iter()
                    .map(|elem| match elem {
                        crate::grue_compiler::ast::PatternElement::Literal(s) => {
                            IrPatternElement::Literal(s)
                        }
                        crate::grue_compiler::ast::PatternElement::Noun => IrPatternElement::Noun,
                        crate::grue_compiler::ast::PatternElement::Default => {
                            IrPatternElement::Default
                        }
                        // Enhanced parser elements - full support for Zork I-level parsing
                        crate::grue_compiler::ast::PatternElement::Adjective => {
                            IrPatternElement::Adjective
                        }
                        crate::grue_compiler::ast::PatternElement::MultiWordNoun => {
                            IrPatternElement::MultiWordNoun
                        }
                        crate::grue_compiler::ast::PatternElement::Preposition => {
                            IrPatternElement::Preposition
                        }
                        crate::grue_compiler::ast::PatternElement::MultipleObjects => {
                            IrPatternElement::MultipleObjects
                        }
                        crate::grue_compiler::ast::PatternElement::DirectObject => {
                            IrPatternElement::DirectObject
                        }
                        crate::grue_compiler::ast::PatternElement::IndirectObject => {
                            IrPatternElement::IndirectObject
                        }
                        crate::grue_compiler::ast::PatternElement::OptionalAdjective => {
                            IrPatternElement::OptionalAdjective
                        }
                        crate::grue_compiler::ast::PatternElement::AnyPreposition => {
                            IrPatternElement::AnyPreposition
                        }
                        crate::grue_compiler::ast::PatternElement::NumberedNoun => {
                            IrPatternElement::NumberedNoun
                        }
                    })
                    .collect();

                let ir_handler = match pattern.handler {
                    crate::grue_compiler::ast::Handler::FunctionCall(name, args) => {
                        // Convert arguments to IR values
                        let mut ir_args = Vec::new();
                        for arg in args {
                            let ir_value = self.expr_to_ir_value(arg)?;
                            ir_args.push(ir_value);
                        }

                        // CRITICAL FIX: Look up function ID using symbol table resolution
                        // Previously used placeholder function ID 0, causing "Routine ID 0 not found" errors
                        // during code generation. Now properly resolves function names to their assigned IR IDs.
                        // This enables grammar pattern handlers like handle_look() to be correctly called.
                        let func_id = if let Some(&id) = self.symbol_ids.get(&name) {
                            id
                        } else {
                            return Err(CompilerError::SemanticError(
                                format!(
                                    "Grammar handler function '{}' not found. All functions must be defined before grammar patterns.",
                                    name
                                ),
                                0,
                            ));
                        };

                        IrHandler::FunctionCall(func_id, ir_args)
                    }
                    crate::grue_compiler::ast::Handler::Block(block) => {
                        let ir_block = self.generate_block(block)?;
                        IrHandler::Block(ir_block)
                    }
                };

                patterns.push(IrPattern {
                    pattern: ir_pattern_elements,
                    handler: ir_handler,
                });
            }

            ir_grammar.push(IrGrammar {
                verb: verb.word,
                patterns,
            });
        }

        Ok(ir_grammar)
    }

    fn generate_block(
        &mut self,
        block: crate::grue_compiler::ast::BlockStmt,
    ) -> Result<IrBlock, CompilerError> {
        let block_id = self.next_id();
        let mut ir_block = IrBlock::new(block_id);

        log::debug!(
            "IR generate_block: Processing {} statements",
            block.statements.len()
        );
        for (i, stmt) in block.statements.iter().enumerate() {
            log::debug!(
                "IR generate_block: Processing statement {} of type {:?}",
                i,
                stmt
            );
            self.generate_statement(stmt.clone(), &mut ir_block)?;
        }

        Ok(ir_block)
    }

    fn generate_object_tree_iteration(
        &mut self,
        for_stmt: Box<crate::grue_compiler::ast::ForStmt>,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        use crate::grue_compiler::ast::Expr;

        // Extract the object from the contents() method call
        let container_object = if let Expr::MethodCall { object, .. } = for_stmt.iterable {
            self.generate_expression(*object, block)?
        } else {
            return Err(CompilerError::CodeGenError(
                "Expected MethodCall for object tree iteration".to_string(),
            ));
        };

        self.generate_object_tree_iteration_with_container(
            for_stmt.variable,
            *for_stmt.body,
            container_object,
            block,
        )
    }

    fn generate_object_tree_iteration_with_container(
        &mut self,
        variable: String,
        body: crate::grue_compiler::ast::Stmt,
        container_object: IrId,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        // Create loop variable for current object
        let loop_var_id = self.next_id();
        let local_var = IrLocal {
            ir_id: loop_var_id,
            name: variable.clone(),
            var_type: Some(Type::Any),
            slot: self.next_local_slot,
            mutable: false,
        };
        self.current_locals.push(local_var);
        self.symbol_ids.insert(variable, loop_var_id);
        self.next_local_slot += 1;

        // Create current_object variable to track iteration
        let current_obj_var = self.next_id();
        let current_obj_local = IrLocal {
            ir_id: current_obj_var,
            name: format!("__current_obj_{}", current_obj_var),
            var_type: Some(Type::Int),
            slot: self.next_local_slot,
            mutable: true,
        };
        self.current_locals.push(current_obj_local);
        self.next_local_slot += 1;

        // Create labels
        let loop_start = self.next_id();
        let loop_body = self.next_id();
        let loop_end = self.next_id();

        // Get first child: current = get_child(container)
        // Z-Machine GET_CHILD branches when there's NO child (returns 0)
        let first_child_temp = self.next_id();
        block.add_instruction(IrInstruction::GetObjectChild {
            target: first_child_temp,
            object: container_object,
            branch_if_no_child: loop_end, // Skip loop if container has no children
        });
        block.add_instruction(IrInstruction::StoreVar {
            var_id: current_obj_var,
            source: first_child_temp,
        });

        // Loop start label (we continue here after processing each child)
        block.add_instruction(IrInstruction::Label { id: loop_start });

        // Loop body: set loop variable = current object
        block.add_instruction(IrInstruction::Label { id: loop_body });
        let current_for_body = self.next_id();
        block.add_instruction(IrInstruction::LoadVar {
            target: current_for_body,
            var_id: current_obj_var,
        });
        block.add_instruction(IrInstruction::StoreVar {
            var_id: loop_var_id,
            source: current_for_body,
        });

        // Execute loop body
        self.generate_statement(body, block)?;

        // Get next sibling: current = get_sibling(current)
        // Z-Machine GET_SIBLING branches when there's NO sibling (returns 0)
        let current_for_sibling = self.next_id();
        block.add_instruction(IrInstruction::LoadVar {
            target: current_for_sibling,
            var_id: current_obj_var,
        });
        let next_sibling_temp = self.next_id();
        block.add_instruction(IrInstruction::GetObjectSibling {
            target: next_sibling_temp,
            object: current_for_sibling,
            branch_if_no_sibling: loop_end, // Exit loop when no more siblings
        });
        block.add_instruction(IrInstruction::StoreVar {
            var_id: current_obj_var,
            source: next_sibling_temp,
        });

        // Jump back to start to process next sibling
        block.add_instruction(IrInstruction::Jump { label: loop_start });

        // Loop end
        block.add_instruction(IrInstruction::Label { id: loop_end });

        Ok(())
    }

    /// Generate InsertObj instructions to place room objects in their containing rooms
    /// Phase 1: Place objects defined inside rooms (e.g., mailbox in west_of_house)
    fn generate_room_object_placement(
        &mut self,
        _block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        log::debug!("🏠 Generating room object placement instructions");

        // We need access to room data, but it's not stored in self after generation
        // For now, implement a simple approach: track room->objects during generation

        // TODO: Implement room object placement logic
        log::warn!("🚧 generate_room_object_placement: Not yet implemented");

        Ok(())
    }

    fn generate_statement(
        &mut self,
        stmt: crate::grue_compiler::ast::Stmt,
        block: &mut IrBlock,
    ) -> Result<(), CompilerError> {
        use crate::grue_compiler::ast::Stmt;

        match stmt {
            Stmt::Expression(expr) => {
                let _temp = self.generate_expression(expr, block)?;
                // Expression result is discarded
            }
            Stmt::VarDecl(var_decl) => {
                // Generate IR for variable declaration
                let var_id = self.next_id();

                // Add to local variables
                let local_var = IrLocal {
                    ir_id: var_id,
                    name: var_decl.name.clone(),
                    var_type: var_decl.var_type,
                    slot: self.next_local_slot,
                    mutable: var_decl.mutable,
                };
                self.current_locals.push(local_var);
                self.symbol_ids.insert(var_decl.name, var_id);
                self.next_local_slot += 1;

                // Generate initializer if present
                if let Some(initializer) = var_decl.initializer {
                    let init_temp = self.generate_expression(initializer, block)?;
                    block.add_instruction(IrInstruction::StoreVar {
                        var_id,
                        source: init_temp,
                    });

                    // Copy variable source from initializer to variable for iteration tracking
                    // This enables for-loops to detect object tree iteration even with variable assignments
                    if let Some(source) = self.variable_sources.get(&init_temp).cloned() {
                        log::debug!(
                            "VarDecl: copying variable source {:?} from init_temp {} to var_id {}",
                            source,
                            init_temp,
                            var_id
                        );
                        self.variable_sources.insert(var_id, source);
                    }
                }
            }
            Stmt::Assignment(assign) => {
                // Generate the value expression
                let value_temp = self.generate_expression(assign.value, block)?;

                // Handle different types of assignment targets
                match assign.target {
                    crate::grue_compiler::ast::Expr::Identifier(var_name) => {
                        // Simple variable assignment
                        if let Some(&var_id) = self.symbol_ids.get(&var_name) {
                            block.add_instruction(IrInstruction::StoreVar {
                                var_id,
                                source: value_temp,
                            });

                            // CRITICAL FIX (Oct 27, 2025): Copy variable source tracking from value_temp to var_id
                            // This enables for-loop detection to work with variable indirection
                            // e.g., let items = obj.contents(); for item in items
                            // Without this, ObjectTreeRoot source is lost during assignment and for-loop
                            // falls back to array iteration instead of object tree iteration
                            if let Some(source) = self.variable_sources.get(&value_temp).cloned() {
                                log::debug!(
                                    "Assignment: copying variable source {:?} from value_temp {} to var_id {}",
                                    source,
                                    value_temp,
                                    var_id
                                );
                                self.variable_sources.insert(var_id, source);
                            }
                        } else {
                            // Variable not found - this should be caught in semantic analysis
                            return Err(CompilerError::SemanticError(
                                format!("Undefined variable '{}' in assignment", var_name),
                                0,
                            ));
                        }
                    }
                    crate::grue_compiler::ast::Expr::PropertyAccess { object, property } => {
                        // Property assignment: object.property = value
                        let object_temp = self.generate_expression(*object, block)?;

                        // Special handling for .location assignment - use insert_obj instead of property
                        // (Oct 12, 2025): Location is object tree containment only, not a property
                        if property == "location" {
                            log::debug!(
                                "🏃 LOCATION_WRITE: Using InsertObj for .location assignment"
                            );
                            block.add_instruction(IrInstruction::InsertObj {
                                object: object_temp,
                                destination: value_temp,
                            });
                        } else if let Some(standard_prop) = self.get_standard_property(&property) {
                            // Check if this is a standard property that should use numbered access
                            if let Some(prop_num) = self
                                .property_manager
                                .get_standard_property_number(standard_prop)
                            {
                                block.add_instruction(IrInstruction::SetPropertyByNumber {
                                    object: object_temp,
                                    property_num: prop_num,
                                    value: value_temp,
                                });
                            } else {
                                // Use dynamic property manager to assign property number even for standard properties without numbers
                                let prop_num = self.property_manager.get_property_number(&property);
                                block.add_instruction(IrInstruction::SetPropertyByNumber {
                                    object: object_temp,
                                    property_num: prop_num,
                                    value: value_temp,
                                });
                            }
                        } else {
                            // Use dynamic property manager to assign property number for non-standard properties
                            let prop_num = self.property_manager.get_property_number(&property);
                            block.add_instruction(IrInstruction::SetPropertyByNumber {
                                object: object_temp,
                                property_num: prop_num,
                                value: value_temp,
                            });
                        }
                    }
                    _ => {
                        // Other assignment targets (array elements, etc.)
                        return Err(CompilerError::SemanticError(
                            "Unsupported assignment target".to_string(),
                            0,
                        ));
                    }
                }
            }
            Stmt::If(if_stmt) => {
                // Generate condition expression
                let condition_temp = self.generate_expression(if_stmt.condition, block)?;

                log::debug!("IF condition temp: {}", condition_temp);

                // Create labels for control flow
                let then_label = self.next_id();
                let else_label = self.next_id();
                let end_label = self.next_id();

                log::debug!(
                    "IR if statement: then={}, else={}, end={}",
                    then_label,
                    else_label,
                    end_label
                );

                // Branch based on condition
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label: then_label,
                    false_label: else_label,
                });

                // Then branch
                log::debug!("IR if: Adding then label {}", then_label);
                block.add_instruction(IrInstruction::Label { id: then_label });
                self.generate_statement(*if_stmt.then_branch, block)?;

                // Only emit jump to end_label if there's an else branch
                // Without else branch, fall-through naturally reaches end_label
                if if_stmt.else_branch.is_some() {
                    log::debug!(
                        "IR if: Adding jump to end label {} (else branch exists)",
                        end_label
                    );
                    block.add_instruction(IrInstruction::Jump { label: end_label });
                } else {
                    log::debug!(
                        "IR if: Skipping jump to end label {} (no else branch - fall-through)",
                        end_label
                    );
                }

                // Else branch (if present)
                log::debug!("IR if: Adding else label {}", else_label);
                block.add_instruction(IrInstruction::Label { id: else_label });
                if let Some(else_branch) = if_stmt.else_branch {
                    self.generate_statement(*else_branch, block)?;
                }

                // End label
                log::debug!("IR if: Adding end label {}", end_label);
                block.add_instruction(IrInstruction::Label { id: end_label });
            }
            Stmt::While(while_stmt) => {
                // Create labels for loop control flow
                let loop_start = self.next_id();
                let loop_body = self.next_id();
                let loop_end = self.next_id();

                // Jump to loop start
                block.add_instruction(IrInstruction::Jump { label: loop_start });

                // Loop start: evaluate condition
                block.add_instruction(IrInstruction::Label { id: loop_start });
                let condition_temp = self.generate_expression(while_stmt.condition, block)?;

                // Branch based on condition
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label: loop_body,
                    false_label: loop_end,
                });

                // Loop body
                block.add_instruction(IrInstruction::Label { id: loop_body });
                self.generate_statement(*while_stmt.body, block)?;
                block.add_instruction(IrInstruction::Jump { label: loop_start });

                // Loop end
                block.add_instruction(IrInstruction::Label { id: loop_end });
            }
            Stmt::For(for_stmt) => {
                // For loops in Grue iterate over collections
                // Generate the iterable expression first
                let iterable_temp = self.generate_expression(for_stmt.iterable, block)?;

                // Use variable source tracking to determine iteration strategy
                // This handles variable indirection (e.g., let items = obj.contents(); for item in items)
                let container_object =
                    self.variable_sources
                        .get(&iterable_temp)
                        .and_then(|source| {
                            if let VariableSource::ObjectTreeRoot(container_id) = source {
                                Some(*container_id)
                            } else {
                                None
                            }
                        });

                if let Some(container_id) = container_object {
                    // Generate object tree iteration using get_child/get_sibling opcodes
                    return self.generate_object_tree_iteration_with_container(
                        for_stmt.variable,
                        *for_stmt.body,
                        container_id,
                        block,
                    );
                }

                // Otherwise, generate array iteration using get_array_element

                // Create a loop variable
                let loop_var_id = self.next_id();
                let local_var = IrLocal {
                    ir_id: loop_var_id,
                    name: for_stmt.variable.clone(),
                    var_type: Some(Type::Any), // Type inferred from array elements
                    slot: self.next_local_slot,
                    mutable: false, // Loop variables are immutable
                };
                self.current_locals.push(local_var);
                self.symbol_ids.insert(for_stmt.variable, loop_var_id);
                self.next_local_slot += 1;

                // Create index variable for array iteration (allocate as local)
                let index_var = self.next_id();
                let index_local = IrLocal {
                    ir_id: index_var,
                    name: format!("__loop_index_{}", index_var),
                    var_type: Some(Type::Int),
                    slot: self.next_local_slot,
                    mutable: true, // Index is incremented
                };
                self.current_locals.push(index_local);
                self.next_local_slot += 1;

                // Initialize index to 0
                let zero_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: zero_temp,
                    value: IrValue::Integer(0),
                });
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: index_var,
                    source: zero_temp,
                });

                // Create labels for loop control flow
                let loop_start = self.next_id();
                let loop_body = self.next_id();
                let loop_end = self.next_id();

                // Loop start: check if index < array length
                block.add_instruction(IrInstruction::Label { id: loop_start });
                let index_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadVar {
                    target: index_temp,
                    var_id: index_var,
                });

                // CRITICAL FIX: Implement single-iteration loop for placeholder arrays
                // The contents() method returns a placeholder value, not a real array
                // So we should iterate exactly once with our placeholder object (player = 1)
                // Compare index with 1 to terminate after first iteration
                let one_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: one_temp,
                    value: IrValue::Integer(1), // Array length = 1 (single placeholder object)
                });

                // Compare index < array_length (1)
                let condition_temp = self.next_id();
                block.add_instruction(IrInstruction::BinaryOp {
                    target: condition_temp,
                    op: IrBinaryOp::Less,
                    left: index_temp,
                    right: one_temp,
                });

                // Branch based on condition
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label: loop_body,
                    false_label: loop_end,
                });

                // Loop body: load current element into loop variable
                block.add_instruction(IrInstruction::Label { id: loop_body });
                // CRITICAL: Reload index since index_temp was consumed by Less comparison
                // This prevents SSA violation (reusing consumed stack value)
                let index_for_get = self.next_id();
                block.add_instruction(IrInstruction::LoadVar {
                    target: index_for_get,
                    var_id: index_var,
                });
                let element_temp = self.next_id();
                block.add_instruction(IrInstruction::GetArrayElement {
                    target: element_temp,
                    array: iterable_temp,
                    index: index_for_get,
                });
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: loop_var_id,
                    source: element_temp,
                });

                // Execute loop body
                self.generate_statement(*for_stmt.body, block)?;

                // Increment index
                // CRITICAL: Reload index_var since index_temp was consumed by GetArrayElement
                // This prevents SSA violation (reusing consumed stack value)
                let index_for_increment = self.next_id();
                block.add_instruction(IrInstruction::LoadVar {
                    target: index_for_increment,
                    var_id: index_var,
                });
                let one_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: one_temp,
                    value: IrValue::Integer(1),
                });
                let new_index = self.next_id();
                block.add_instruction(IrInstruction::BinaryOp {
                    target: new_index,
                    op: IrBinaryOp::Add,
                    left: index_for_increment,
                    right: one_temp,
                });
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: index_var,
                    source: new_index,
                });

                // Jump back to start
                block.add_instruction(IrInstruction::Jump { label: loop_start });

                // Loop end
                block.add_instruction(IrInstruction::Label { id: loop_end });
            }
            Stmt::Return(return_expr) => {
                let value_id = if let Some(expr) = return_expr {
                    Some(self.generate_expression(expr, block)?)
                } else {
                    None
                };

                block.add_instruction(IrInstruction::Return { value: value_id });
            }
            Stmt::Block(inner_block) => {
                let ir_inner_block = self.generate_block(inner_block)?;
                // Inline the inner block's instructions
                block.instructions.extend(ir_inner_block.instructions);
            }
        }

        Ok(())
    }

    fn generate_expression(
        &mut self,
        expr: crate::grue_compiler::ast::Expr,
        block: &mut IrBlock,
    ) -> Result<IrId, CompilerError> {
        use crate::grue_compiler::ast::Expr;

        match expr {
            Expr::Integer(value) => {
                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::Integer(value),
                });
                Ok(temp_id)
            }
            Expr::String(value) => {
                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::String(value),
                });
                Ok(temp_id)
            }
            Expr::Boolean(value) => {
                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::Boolean(value),
                });
                Ok(temp_id)
            }
            Expr::Identifier(name) => {
                // CRITICAL ARCHITECTURAL FIX (Sep 13, 2025): Handle player object specially
                //
                // PROBLEM: Player object references were being generated as LoadImmediate(1) → LargeConstant(1)
                // causing stack underflow in get_prop instructions that expected Variable(16).
                //
                // SOLUTION: Player object must be read from Global G00 (Variable 16) per Z-Machine spec.
                // This ensures proper distinction between:
                // - Literal integer 1 → LargeConstant(1)
                // - Player object reference → Variable(16) (reads from Global G00)
                //
                // This fixes the architectural issue where player.location calls generated wrong operand types.
                if name == "player" {
                    log::debug!("🏃 IR_FIX: Generating LoadVar for player object (will read from Global G00)");
                    let temp_id = self.next_id();
                    block.add_instruction(IrInstruction::LoadVar {
                        target: temp_id,
                        var_id: 16, // Global G00 = Variable 16 = player object number
                    });
                    Ok(temp_id)
                } else if let Some(&object_number) = self.object_numbers.get(&name) {
                    // This is a regular object (not player) - load its number as a constant
                    let temp_id = self.next_id();
                    block.add_instruction(IrInstruction::LoadImmediate {
                        target: temp_id,
                        value: IrValue::Integer(object_number as i16),
                    });
                    Ok(temp_id)
                } else if let Some(&var_id) = self.symbol_ids.get(&name) {
                    // This is an existing variable - return its original ID directly
                    // No need to create LoadVar instruction since the variable already exists
                    log::debug!(
                        "✅ IR_FIX: Reusing existing variable ID {} for '{}'",
                        var_id,
                        name
                    );
                    Ok(var_id)
                } else {
                    // Identifier not found - this should be caught during semantic analysis
                    Err(CompilerError::SemanticError(
                        format!("Undefined identifier '{}'", name),
                        0,
                    ))
                }
            }
            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left_id = self.generate_expression(*left, block)?;
                let right_id = self.generate_expression(*right, block)?;
                let temp_id = self.next_id();

                block.add_instruction(IrInstruction::BinaryOp {
                    target: temp_id,
                    op: operator.into(),
                    left: left_id,
                    right: right_id,
                });

                Ok(temp_id)
            }
            Expr::Unary { operator, operand } => {
                let operand_id = self.generate_expression(*operand, block)?;
                let temp_id = self.next_id();

                block.add_instruction(IrInstruction::UnaryOp {
                    target: temp_id,
                    op: operator.into(),
                    operand: operand_id,
                });

                Ok(temp_id)
            }
            Expr::FunctionCall { name, arguments } => {
                // Generate arguments first
                let mut arg_temps = Vec::new();
                for arg in arguments {
                    let arg_temp = self.generate_expression(arg, block)?;
                    arg_temps.push(arg_temp);
                }

                // Check if this is a built-in function that needs special IR handling
                if self.is_builtin_function(&name) {
                    return self.generate_builtin_function_call(&name, &arg_temps, block);
                }

                // Look up function ID (should be pre-registered)
                let func_id = if let Some(&id) = self.symbol_ids.get(&name) {
                    id
                } else {
                    return Err(CompilerError::SemanticError(
                        format!(
                            "Function '{}' not found. All functions must be defined before use.",
                            name
                        ),
                        0,
                    ));
                };

                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::Call {
                    target: Some(temp_id), // Assume all function calls return something
                    function: func_id,
                    args: arg_temps,
                });

                Ok(temp_id)
            }
            Expr::MethodCall {
                object,
                method,
                arguments,
            } => {
                // Check if this is an array method call before moving the object
                let is_array = self.is_array_type(&object);

                // Generate object expression
                let object_temp = self.generate_expression(*object, block)?;

                // For array methods, generate built-in operations instead of property-based calls
                if is_array {
                    return self.generate_array_method_call(
                        object_temp,
                        &method,
                        &arguments,
                        block,
                    );
                }

                // Method call: object.method(args)

                // Generate arguments first
                let mut arg_temps = Vec::new();
                for arg in arguments {
                    let arg_temp = self.generate_expression(arg, block)?;
                    arg_temps.push(arg_temp);
                }

                // Check if this is a known built-in pseudo-method that doesn't require property lookup
                let is_builtin_pseudo_method =
                    matches!(method.as_str(), "get_exit" | "empty" | "none" | "contents");

                if is_builtin_pseudo_method {
                    // For built-in pseudo-methods, generate direct call without property check
                    let result_temp = self.next_id();

                    match method.as_str() {
                        "get_exit" => {
                            let builtin_id = self.next_id();
                            self.builtin_functions
                                .insert(builtin_id, "get_exit".to_string());

                            let mut call_args = vec![object_temp];
                            call_args.extend(arg_temps);

                            block.add_instruction(IrInstruction::Call {
                                target: Some(result_temp),
                                function: builtin_id,
                                args: call_args,
                            });
                        }
                        "empty" => {
                            let builtin_id = self.next_id();
                            self.builtin_functions
                                .insert(builtin_id, "object_is_empty".to_string());

                            let mut call_args = vec![object_temp];
                            call_args.extend(arg_temps);

                            block.add_instruction(IrInstruction::Call {
                                target: Some(result_temp),
                                function: builtin_id,
                                args: call_args,
                            });
                        }
                        "none" => {
                            let builtin_id = self.next_id();
                            self.builtin_functions
                                .insert(builtin_id, "value_is_none".to_string());

                            let mut call_args = vec![object_temp];
                            call_args.extend(arg_temps);

                            block.add_instruction(IrInstruction::Call {
                                target: Some(result_temp),
                                function: builtin_id,
                                args: call_args,
                            });
                        }
                        "contents" => {
                            let builtin_id = self.next_id();
                            self.builtin_functions
                                .insert(builtin_id, "get_object_contents".to_string());

                            let call_args = vec![object_temp];

                            block.add_instruction(IrInstruction::Call {
                                target: Some(result_temp),
                                function: builtin_id,
                                args: call_args,
                            });

                            // Track contents() results as object tree roots for iteration
                            // This enables for-loops to detect object tree iteration even with variable indirection
                            self.variable_sources
                                .insert(result_temp, VariableSource::ObjectTreeRoot(object_temp));
                            log::debug!(
                                "Builtin contents(): tracking result_temp {} as ObjectTreeRoot({})",
                                result_temp,
                                object_temp
                            );
                        }
                        _ => unreachable!(),
                    }

                    return Ok(result_temp);
                }

                // For regular property-based methods, generate property lookup and conditional call

                // Check if this is contents() for object tree iteration tracking
                let is_contents_method = method.as_str() == "contents";

                // Generate property access to get the method function
                let property_temp = self.next_id();
                let prop_num = self.property_manager.get_property_number(&method);
                block.add_instruction(IrInstruction::GetPropertyByNumber {
                    target: property_temp,
                    object: object_temp,
                    property_num: prop_num,
                });

                // Generate conditional call - only call if property is non-zero (valid function address)
                let result_temp = self.next_id();
                let then_label = self.next_id();
                let else_label = self.next_id();
                let end_label = self.next_id();

                // Branch: if property_temp != 0, goto then_label, else goto else_label
                block.add_instruction(IrInstruction::Branch {
                    condition: property_temp,
                    true_label: then_label,
                    false_label: else_label,
                });

                // Then branch: call the function stored in the property
                block.add_instruction(IrInstruction::Label { id: then_label });

                // Special handling for property-based methods that have fallback behavior
                match method.as_str() {
                    "size" | "length" => {
                        // size() or length() method: return count of elements/contents
                        let builtin_id = self.next_id();
                        self.builtin_functions
                            .insert(builtin_id, "get_object_size".to_string());

                        let mut call_args = vec![object_temp];
                        call_args.extend(arg_temps);

                        block.add_instruction(IrInstruction::Call {
                            target: Some(result_temp),
                            function: builtin_id,
                            args: call_args,
                        });
                    }
                    "add" => {
                        // Array/collection add method - for arrays like visible_objects.add(obj)
                        // Implement as proper builtin function call instead of LoadImmediate fallback
                        let builtin_id = self.next_id();
                        self.builtin_functions
                            .insert(builtin_id, "array_add_item".to_string());

                        let mut call_args = vec![object_temp];
                        call_args.extend(arg_temps);

                        block.add_instruction(IrInstruction::Call {
                            target: Some(result_temp),
                            function: builtin_id,
                            args: call_args,
                        });
                    }
                    "on_enter" | "on_exit" | "on_look" => {
                        // Object handler methods - these call property-based function handlers
                        // In Grue, these are properties that contain function addresses
                        // The pattern is: if object.property exists, call it as a function

                        // Get property number for this handler - this will register it if not found
                        let property_name = method;
                        let property_number =
                            self.property_manager.get_property_number(&property_name);

                        // Use proper property-based function call
                        block.add_instruction(IrInstruction::GetPropertyByNumber {
                            target: result_temp,
                            object: object_temp,
                            property_num: property_number,
                        });

                        // TODO: In a complete implementation, this would:
                        // 1. Get the property value (function address)
                        // 2. Check if it's non-zero (function exists)
                        // 3. Call the function if it exists
                        // For now, we'll return the property value directly

                        log::debug!(
                            "Object handler '{}' mapped to property #{} for method call",
                            property_name,
                            property_number
                        );
                    }
                    _ => {
                        // For truly unknown methods, return safe non-zero value to prevent object 0 errors
                        // This prevents "Cannot insert object 0" crashes while providing a detectable placeholder
                        log::warn!("Unknown method '{}' called on object - returning safe placeholder value 1", method);
                        block.add_instruction(IrInstruction::LoadImmediate {
                            target: result_temp,
                            value: IrValue::Integer(1), // Safe non-zero value instead of 0
                        });
                    }
                }

                block.add_instruction(IrInstruction::Jump { label: end_label });

                // Else branch: property doesn't exist or isn't callable, return safe non-zero value
                block.add_instruction(IrInstruction::Label { id: else_label });
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: result_temp,
                    value: IrValue::Integer(1), // Use 1 instead of 0 to prevent null operands
                });

                // End label
                block.add_instruction(IrInstruction::Label { id: end_label });

                // Track contents() results as object tree roots for iteration
                // This enables for-loops to detect object tree iteration even with variable indirection
                if is_contents_method {
                    self.variable_sources
                        .insert(result_temp, VariableSource::ObjectTreeRoot(object_temp));
                }

                Ok(result_temp)
            }
            Expr::PropertyAccess { object, property } => {
                // Property access: object.property
                let is_array = self.is_array_type(&object);
                let object_temp = self.generate_expression(*object, block)?;
                let temp_id = self.next_id();

                log::debug!(
                    "🔍 PropertyAccess: property='{}', object_temp={}",
                    property,
                    object_temp
                );

                // Check if this is an exit value property access (bit manipulation)
                // Exit values are encoded as: (type << 14) | data
                // where type=0 for normal exits, type=1 for blocked exits
                match property.as_str() {
                    "blocked" => {
                        // Check if bit 14 is set (value >= 0x4000)
                        let builtin_id = self.next_id();
                        self.builtin_functions
                            .insert(builtin_id, "exit_is_blocked".to_string());

                        log::debug!(
                            "🚪 EXIT: Adding Call instruction: target={}, function={}, args=[{}]",
                            temp_id,
                            builtin_id,
                            object_temp
                        );
                        block.add_instruction(IrInstruction::Call {
                            target: Some(temp_id),
                            function: builtin_id,
                            args: vec![object_temp],
                        });
                        log::debug!(
                            "🚪 EXIT: Call instruction added, returning temp_id={}",
                            temp_id
                        );
                        return Ok(temp_id);
                    }
                    "destination" => {
                        // Extract lower 14 bits (value & 0x3FFF) -> room ID
                        log::debug!("🚪 EXIT: Creating exit_get_destination builtin");

                        let builtin_id = self.next_id();
                        self.builtin_functions
                            .insert(builtin_id, "exit_get_destination".to_string());

                        log::debug!(
                            "🚪 EXIT: Adding Call instruction: target={}, function={}, args=[{}]",
                            temp_id,
                            builtin_id,
                            object_temp
                        );
                        block.add_instruction(IrInstruction::Call {
                            target: Some(temp_id),
                            function: builtin_id,
                            args: vec![object_temp],
                        });

                        log::debug!(
                            "🚪 EXIT: Call instruction added, returning temp_id={}",
                            temp_id
                        );
                        return Ok(temp_id);
                    }
                    "message" => {
                        // Extract lower 14 bits (value & 0x3FFF) -> string address
                        log::debug!("🚪 EXIT: Creating exit_get_message builtin");

                        let builtin_id = self.next_id();
                        self.builtin_functions
                            .insert(builtin_id, "exit_get_message".to_string());

                        log::debug!(
                            "🚪 EXIT: Adding Call instruction: target={}, function={}, args=[{}]",
                            temp_id,
                            builtin_id,
                            object_temp
                        );
                        block.add_instruction(IrInstruction::Call {
                            target: Some(temp_id),
                            function: builtin_id,
                            args: vec![object_temp],
                        });

                        log::debug!(
                            "🚪 EXIT: Call instruction added, returning temp_id={}",
                            temp_id
                        );
                        return Ok(temp_id);
                    }
                    _ => {} // Fall through to normal property access
                }

                // Check if this is an array property access
                if is_array {
                    match property.as_str() {
                        "length" | "size" => {
                            block.add_instruction(IrInstruction::ArrayLength {
                                target: temp_id,
                                array: object_temp,
                            });
                            return Ok(temp_id);
                        }
                        _ => {
                            return Err(CompilerError::CodeGenError(format!(
                                "Unknown array property: {}",
                                property
                            )));
                        }
                    }
                }

                // Special handling for .location - use get_parent instead of property access
                // BUG FIX (Oct 11, 2025): player.location must read parent from object tree,
                // not from a property, because move() uses insert_obj which updates the tree
                if property == "location" {
                    log::debug!(
                        "🏃 LOCATION_FIX: Using GetObjectParent for .location property access"
                    );
                    block.add_instruction(IrInstruction::GetObjectParent {
                        target: temp_id,
                        object: object_temp,
                    });
                } else if let Some(standard_prop) = self.get_standard_property(&property) {
                    // Check if this is a standard property that should use numbered access
                    if let Some(prop_num) = self
                        .property_manager
                        .get_standard_property_number(standard_prop)
                    {
                        block.add_instruction(IrInstruction::GetPropertyByNumber {
                            target: temp_id,
                            object: object_temp,
                            property_num: prop_num,
                        });
                    } else {
                        // Use dynamic property manager to assign property number even for standard properties without numbers
                        let prop_num = self.property_manager.get_property_number(&property);
                        block.add_instruction(IrInstruction::GetPropertyByNumber {
                            target: temp_id,
                            object: object_temp,
                            property_num: prop_num,
                        });
                    }
                } else {
                    // Use dynamic property manager to assign property number for non-standard properties
                    let prop_num = self.property_manager.get_property_number(&property);
                    block.add_instruction(IrInstruction::GetPropertyByNumber {
                        target: temp_id,
                        object: object_temp,
                        property_num: prop_num,
                    });
                }

                log::debug!(
                    "Property access: object={} property='{}' -> temp={}",
                    object_temp,
                    property,
                    temp_id
                );

                Ok(temp_id)
            }

            Expr::NullSafePropertyAccess { object, property } => {
                // Null-safe property access: object?.property
                let is_array = self.is_array_type(&object);
                let object_temp = self.generate_expression(*object, block)?;
                let temp_id = self.next_id();

                // For null-safe access, we need to check if the object is null/valid first
                let null_check_label = self.next_id();
                let valid_label = self.next_id();
                let end_label = self.next_id();

                // Check if object is null (0)
                let zero_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: zero_temp,
                    value: IrValue::Integer(0),
                });

                // Compare object with zero
                let condition_temp = self.next_id();
                block.add_instruction(IrInstruction::BinaryOp {
                    target: condition_temp,
                    op: IrBinaryOp::NotEqual,
                    left: object_temp,
                    right: zero_temp,
                });

                // Branch: if object != 0, goto valid_label, else goto null_check_label
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label: valid_label,
                    false_label: null_check_label,
                });

                // Null case: return null/0
                block.add_instruction(IrInstruction::Label {
                    id: null_check_label,
                });
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::Integer(0),
                });
                block.add_instruction(IrInstruction::Jump { label: end_label });

                // Valid case: perform normal property access
                block.add_instruction(IrInstruction::Label { id: valid_label });
                if is_array {
                    match property.as_str() {
                        "length" | "size" => {
                            block.add_instruction(IrInstruction::ArrayLength {
                                target: temp_id,
                                array: object_temp,
                            });
                        }
                        _ => {
                            return Err(CompilerError::CodeGenError(format!(
                                "Unknown array property: {}",
                                property
                            )));
                        }
                    }
                } else {
                    // Check if this is a standard property that should use numbered access
                    if let Some(standard_prop) = self.get_standard_property(&property) {
                        if let Some(prop_num) = self
                            .property_manager
                            .get_standard_property_number(standard_prop)
                        {
                            block.add_instruction(IrInstruction::GetPropertyByNumber {
                                target: temp_id,
                                object: object_temp,
                                property_num: prop_num,
                            });
                        } else {
                            // Use dynamic property manager to assign property number even for standard properties without numbers
                            let prop_num = self.property_manager.get_property_number(&property);
                            block.add_instruction(IrInstruction::GetPropertyByNumber {
                                target: temp_id,
                                object: object_temp,
                                property_num: prop_num,
                            });
                        }
                    } else {
                        // Use dynamic property manager to assign property number for non-standard properties
                        let prop_num = self.property_manager.get_property_number(&property);
                        block.add_instruction(IrInstruction::GetPropertyByNumber {
                            target: temp_id,
                            object: object_temp,
                            property_num: prop_num,
                        });
                    }
                }

                // End label
                block.add_instruction(IrInstruction::Label { id: end_label });

                Ok(temp_id)
            }

            Expr::Array(elements) => {
                // Array literal - create array and populate with elements
                let array_size = elements.len() as i16; // Save size before elements is moved

                // Generate expression for each element
                let mut element_temps = Vec::new();
                for element in elements {
                    let element_temp = self.generate_expression(element, block)?;
                    element_temps.push(element_temp);
                }

                // Create the array with the determined size
                let array_temp = self.next_id();
                block.add_instruction(IrInstruction::CreateArray {
                    target: array_temp,
                    size: IrValue::Integer(array_size),
                });

                // Track that this is an array literal
                self.variable_sources
                    .insert(array_temp, VariableSource::Array(array_temp));

                // Populate array with elements
                for (index, element_id) in element_temps.iter().enumerate() {
                    let index_temp = self.next_id();
                    block.add_instruction(IrInstruction::LoadImmediate {
                        target: index_temp,
                        value: IrValue::Integer(index as i16),
                    });
                    block.add_instruction(IrInstruction::SetArrayElement {
                        array: array_temp,
                        index: index_temp,
                        value: *element_id,
                    });
                }

                Ok(array_temp)
            }
            Expr::Ternary {
                condition,
                true_expr,
                false_expr,
            } => {
                // Ternary conditional: condition ? true_expr : false_expr
                let condition_temp = self.generate_expression(*condition, block)?;

                // Create labels for control flow
                let true_label = self.next_id();
                let false_label = self.next_id();
                let end_label = self.next_id();
                let result_temp = self.next_id();

                // Branch based on condition
                block.add_instruction(IrInstruction::Branch {
                    condition: condition_temp,
                    true_label,
                    false_label,
                });

                // True branch
                block.add_instruction(IrInstruction::Label { id: true_label });
                let true_temp = self.generate_expression(*true_expr, block)?;
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: result_temp,
                    source: true_temp,
                });
                block.add_instruction(IrInstruction::Jump { label: end_label });

                // False branch
                block.add_instruction(IrInstruction::Label { id: false_label });
                let false_temp = self.generate_expression(*false_expr, block)?;
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: result_temp,
                    source: false_temp,
                });

                // End label
                block.add_instruction(IrInstruction::Label { id: end_label });

                // Load result
                let final_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadVar {
                    target: final_temp,
                    var_id: result_temp,
                });

                Ok(final_temp)
            }
            Expr::Parameter(param_name) => {
                // Grammar parameter reference (e.g., $noun)
                let temp_id = self.next_id();
                // For now, just create a placeholder
                // In a full implementation, this would reference the parsed parameter
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::String(param_name),
                });
                Ok(temp_id)
            }

            // Enhanced parser expressions (for future Phase 1.3 implementation)
            Expr::ParsedObject {
                adjectives: _,
                noun,
                article: _,
            } => {
                // For now, treat as simple string identifier
                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::String(noun),
                });
                Ok(temp_id)
            }

            Expr::MultipleObjects(objects) => {
                // For now, just use the first object
                if let Some(first_obj) = objects.into_iter().next() {
                    self.generate_expression(first_obj, block)
                } else {
                    let temp_id = self.next_id();
                    block.add_instruction(IrInstruction::LoadImmediate {
                        target: temp_id,
                        value: IrValue::Integer(1), // Use safe non-zero value instead of Null
                    });
                    Ok(temp_id)
                }
            }

            Expr::DisambiguationContext {
                candidates: _,
                query,
            } => {
                // For now, treat as simple string
                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::String(query),
                });
                Ok(temp_id)
            }
        }
    }

    fn expr_to_ir_value(
        &mut self,
        expr: crate::grue_compiler::ast::Expr,
    ) -> Result<IrValue, CompilerError> {
        use crate::grue_compiler::ast::Expr;

        match expr {
            Expr::Integer(value) => Ok(IrValue::Integer(value)),
            Expr::String(value) => Ok(IrValue::String(value)),
            Expr::Boolean(value) => Ok(IrValue::Boolean(value)),
            _ => {
                // For complex expressions, we'd need to generate temporary instructions
                // For now, return a placeholder
                Ok(IrValue::Null)
            }
        }
    }

    /// Check if an expression represents an array type
    fn is_array_type(&self, expr: &crate::grue_compiler::ast::Expr) -> bool {
        use crate::grue_compiler::ast::Expr;
        match expr {
            Expr::Array(_) => true,
            Expr::Identifier(name) => {
                // First, check if this variable is tracked in variable_sources
                // This takes precedence over name-based heuristics
                if let Some(&var_id) = self.symbol_ids.get(name) {
                    if let Some(source) = self.variable_sources.get(&var_id) {
                        return match source {
                            VariableSource::Array(_) => true,           // Explicitly an array
                            VariableSource::ObjectTreeRoot(_) => false, // Contents result - NOT an array
                            VariableSource::Scalar(_) => false, // Scalar value - NOT an array
                        };
                    }
                }

                // Fall back to name-based heuristic for untracked variables
                // Only consider identifiers that are likely to be arrays
                // This is a simplified heuristic - in a full implementation,
                // we'd track variable types through semantic analysis
                name.contains("array")
                    || name.contains("list")
                    || name.contains("items")
                    || name.contains("numbers")
                    || name.contains("strings")
                    || name.contains("elements")
            }
            _ => false,
        }
    }

    /// Generate IR for array method calls
    fn generate_array_method_call(
        &mut self,
        array_temp: IrId,
        method: &str,
        arguments: &[crate::grue_compiler::ast::Expr],
        block: &mut IrBlock,
    ) -> Result<IrId, CompilerError> {
        match method {
            "add" | "push" => {
                if arguments.len() != 1 {
                    return Err(CompilerError::CodeGenError(format!(
                        "Array.{} expects 1 argument",
                        method
                    )));
                }
                let value_temp = self.generate_expression(arguments[0].clone(), block)?;
                block.add_instruction(IrInstruction::ArrayAdd {
                    array: array_temp,
                    value: value_temp,
                });
                // add() doesn't return a value, so return a dummy temp
                let dummy_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: dummy_temp,
                    value: IrValue::Integer(0),
                });
                Ok(dummy_temp)
            }
            "remove" | "removeAt" => {
                if arguments.len() != 1 {
                    return Err(CompilerError::CodeGenError(format!(
                        "Array.{} expects 1 argument",
                        method
                    )));
                }
                let index_temp = self.generate_expression(arguments[0].clone(), block)?;
                let result_temp = self.next_id();
                block.add_instruction(IrInstruction::ArrayRemove {
                    target: result_temp,
                    array: array_temp,
                    index: index_temp,
                });
                Ok(result_temp)
            }
            "length" | "size" => {
                if !arguments.is_empty() {
                    return Err(CompilerError::CodeGenError(format!(
                        "Array.{} expects 0 arguments",
                        method
                    )));
                }
                let result_temp = self.next_id();
                block.add_instruction(IrInstruction::ArrayLength {
                    target: result_temp,
                    array: array_temp,
                });
                Ok(result_temp)
            }
            "empty" | "isEmpty" => {
                if !arguments.is_empty() {
                    return Err(CompilerError::CodeGenError(format!(
                        "Array.{} expects 0 arguments",
                        method
                    )));
                }
                let result_temp = self.next_id();
                block.add_instruction(IrInstruction::ArrayEmpty {
                    target: result_temp,
                    array: array_temp,
                });
                Ok(result_temp)
            }
            "contains" => {
                if arguments.len() != 1 {
                    return Err(CompilerError::CodeGenError(format!(
                        "Array.{} expects 1 argument",
                        method
                    )));
                }
                let value_temp = self.generate_expression(arguments[0].clone(), block)?;
                let result_temp = self.next_id();
                block.add_instruction(IrInstruction::ArrayContains {
                    target: result_temp,
                    array: array_temp,
                    value: value_temp,
                });
                Ok(result_temp)
            }
            "filter" => {
                if arguments.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "Array.filter expects 1 argument".to_string(),
                    ));
                }
                let predicate_temp = self.generate_expression(arguments[0].clone(), block)?;
                let result_temp = self.next_id();
                block.add_instruction(IrInstruction::ArrayFilter {
                    target: result_temp,
                    array: array_temp,
                    predicate: predicate_temp,
                });
                Ok(result_temp)
            }
            "map" => {
                if arguments.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "Array.map expects 1 argument".to_string(),
                    ));
                }
                let transform_temp = self.generate_expression(arguments[0].clone(), block)?;
                let result_temp = self.next_id();
                block.add_instruction(IrInstruction::ArrayMap {
                    target: result_temp,
                    array: array_temp,
                    transform: transform_temp,
                });
                Ok(result_temp)
            }
            "forEach" => {
                if arguments.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "Array.forEach expects 1 argument".to_string(),
                    ));
                }
                let callback_temp = self.generate_expression(arguments[0].clone(), block)?;
                block.add_instruction(IrInstruction::ArrayForEach {
                    array: array_temp,
                    callback: callback_temp,
                });
                // forEach doesn't return a value
                let dummy_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: dummy_temp,
                    value: IrValue::Integer(0),
                });
                Ok(dummy_temp)
            }
            "find" => {
                if arguments.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "Array.find expects 1 argument".to_string(),
                    ));
                }
                let predicate_temp = self.generate_expression(arguments[0].clone(), block)?;
                let result_temp = self.next_id();
                block.add_instruction(IrInstruction::ArrayFind {
                    target: result_temp,
                    array: array_temp,
                    predicate: predicate_temp,
                });
                Ok(result_temp)
            }
            "indexOf" => {
                if arguments.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "Array.indexOf expects 1 argument".to_string(),
                    ));
                }
                let value_temp = self.generate_expression(arguments[0].clone(), block)?;
                let result_temp = self.next_id();
                block.add_instruction(IrInstruction::ArrayIndexOf {
                    target: result_temp,
                    array: array_temp,
                    value: value_temp,
                });
                Ok(result_temp)
            }
            "join" => {
                if arguments.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "Array.join expects 1 argument".to_string(),
                    ));
                }
                let separator_temp = self.generate_expression(arguments[0].clone(), block)?;
                let result_temp = self.next_id();
                block.add_instruction(IrInstruction::ArrayJoin {
                    target: result_temp,
                    array: array_temp,
                    separator: separator_temp,
                });
                Ok(result_temp)
            }
            "reverse" => {
                if !arguments.is_empty() {
                    return Err(CompilerError::CodeGenError(
                        "Array.reverse expects 0 arguments".to_string(),
                    ));
                }
                let result_temp = self.next_id();
                block.add_instruction(IrInstruction::ArrayReverse {
                    target: result_temp,
                    array: array_temp,
                });
                Ok(result_temp)
            }
            "sort" => {
                if arguments.len() > 1 {
                    return Err(CompilerError::CodeGenError(
                        "Array.sort expects 0 or 1 arguments".to_string(),
                    ));
                }
                let comparator = if arguments.is_empty() {
                    None
                } else {
                    Some(self.generate_expression(arguments[0].clone(), block)?)
                };
                let result_temp = self.next_id();
                block.add_instruction(IrInstruction::ArraySort {
                    target: result_temp,
                    array: array_temp,
                    comparator,
                });
                Ok(result_temp)
            }
            _ => Err(CompilerError::CodeGenError(format!(
                "Unknown array method: {}",
                method
            ))),
        }
    }

    fn generate_builtin_function_call(
        &mut self,
        name: &str,
        arg_temps: &[IrId],
        block: &mut IrBlock,
    ) -> Result<IrId, CompilerError> {
        let temp_id = self.next_id();

        match name {
            // String utility functions
            "indexOf" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "indexOf expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringIndexOf {
                    target: temp_id,
                    string: arg_temps[0],
                    substring: arg_temps[1],
                });
            }
            "slice" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "slice expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringSlice {
                    target: temp_id,
                    string: arg_temps[0],
                    start: arg_temps[1],
                });
            }
            "substring" => {
                if arg_temps.len() != 3 {
                    return Err(CompilerError::CodeGenError(
                        "substring expects 3 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringSubstring {
                    target: temp_id,
                    string: arg_temps[0],
                    start: arg_temps[1],
                    end: arg_temps[2],
                });
            }
            "toLowerCase" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "toLowerCase expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringToLowerCase {
                    target: temp_id,
                    string: arg_temps[0],
                });
            }
            "toUpperCase" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "toUpperCase expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringToUpperCase {
                    target: temp_id,
                    string: arg_temps[0],
                });
            }
            "trim" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "trim expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringTrim {
                    target: temp_id,
                    string: arg_temps[0],
                });
            }
            "charAt" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "charAt expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringCharAt {
                    target: temp_id,
                    string: arg_temps[0],
                    index: arg_temps[1],
                });
            }
            "split" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "split expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringSplit {
                    target: temp_id,
                    string: arg_temps[0],
                    delimiter: arg_temps[1],
                });
            }
            "replace" => {
                if arg_temps.len() != 3 {
                    return Err(CompilerError::CodeGenError(
                        "replace expects 3 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringReplace {
                    target: temp_id,
                    string: arg_temps[0],
                    search: arg_temps[1],
                    replacement: arg_temps[2],
                });
            }
            "startsWith" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "startsWith expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringStartsWith {
                    target: temp_id,
                    string: arg_temps[0],
                    prefix: arg_temps[1],
                });
            }
            "endsWith" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "endsWith expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringEndsWith {
                    target: temp_id,
                    string: arg_temps[0],
                    suffix: arg_temps[1],
                });
            }
            // Math utility functions
            "abs" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "abs expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathAbs {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            "min" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "min expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathMin {
                    target: temp_id,
                    a: arg_temps[0],
                    b: arg_temps[1],
                });
            }
            "max" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "max expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathMax {
                    target: temp_id,
                    a: arg_temps[0],
                    b: arg_temps[1],
                });
            }
            "round" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "round expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathRound {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            "floor" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "floor expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathFloor {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            "ceil" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "ceil expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathCeil {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            // Type checking functions
            "is_string" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_string expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "string".to_string(),
                });
            }
            "is_int" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_int expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "int".to_string(),
                });
            }
            "is_bool" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_bool expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "bool".to_string(),
                });
            }
            "is_array" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_array expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "array".to_string(),
                });
            }
            "is_object" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_object expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "object".to_string(),
                });
            }
            "typeof" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "typeof expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeOf {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            // Debug breakpoint (debug builds only)
            #[cfg(debug_assertions)]
            "debug_break" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "debug_break expects 1 argument (label string)".to_string(),
                    ));
                }
                // Extract label from the LoadImmediate instruction
                // We need to look back at the IR to find the string value
                // For now, use a placeholder - we'll need to track this properly
                block.add_instruction(IrInstruction::DebugBreak {
                    label: format!("breakpoint_{}", temp_id),
                });
                // Return a dummy value (0) since debug_break doesn't produce a useful result
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::Integer(0),
                });
            }
            // For other builtin functions, use standard call mechanism
            _ => {
                // Look up function ID (or create placeholder)
                let func_id = if let Some(&id) = self.symbol_ids.get(name) {
                    id
                } else {
                    let placeholder_id = self.next_id();
                    self.symbol_ids.insert(name.to_string(), placeholder_id);
                    self.builtin_functions
                        .insert(placeholder_id, name.to_string());
                    placeholder_id
                };

                block.add_instruction(IrInstruction::Call {
                    target: Some(temp_id),
                    function: func_id,
                    args: arg_temps.to_vec(),
                });
            }
        }

        Ok(temp_id)
    }

    /// Testing method to expose room_objects mapping for integration tests
    #[cfg(test)]
    pub fn get_room_objects(&self) -> &IndexMap<String, Vec<RoomObjectInfo>> {
        &self.room_objects
    }
}

#[cfg(test)]
#[path = "ir_tests.rs"]
mod tests;

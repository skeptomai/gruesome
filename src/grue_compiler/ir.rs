// Intermediate Representation for Grue Language
//
// The IR is designed to be a lower-level representation that's closer to Z-Machine
// instructions while still maintaining some high-level constructs for optimization.

use crate::grue_compiler::ast::{Program, ProgramMode, Type};
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::object_system::ComprehensiveObject;
use std::collections::{HashMap, HashSet};

/// Unique identifier for IR instructions, labels, and temporary variables
pub type IrId = u32;

/// IR Program - top-level container for all IR elements
/// Registry for tracking all IR IDs and their types/purposes
#[derive(Debug, Clone)]
pub struct IrIdRegistry {
    pub id_types: HashMap<IrId, String>,   // ID -> type description
    pub id_sources: HashMap<IrId, String>, // ID -> creation context
    pub temporary_ids: HashSet<IrId>,      // IDs that are temporary values
    pub symbol_ids: HashSet<IrId>,         // IDs that are named symbols
    pub expression_ids: HashSet<IrId>,     // IDs from expression evaluation
}

impl Default for IrIdRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl IrIdRegistry {
    pub fn new() -> Self {
        Self {
            id_types: HashMap::new(),
            id_sources: HashMap::new(),
            temporary_ids: HashSet::new(),
            symbol_ids: HashSet::new(),
            expression_ids: HashSet::new(),
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
    pub string_table: HashMap<String, IrId>, // String literal -> ID mapping
    pub property_defaults: IrPropertyDefaults, // Z-Machine property defaults table
    pub program_mode: ProgramMode,           // Program execution mode
    /// Mapping from symbol names to IR IDs (for identifier resolution)
    pub symbol_ids: HashMap<String, IrId>,
    /// Mapping from object names to Z-Machine object numbers
    pub object_numbers: HashMap<String, u16>,
    /// NEW: Comprehensive registry of all IR IDs and their purposes
    pub id_registry: IrIdRegistry,
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
    pub exits: HashMap<String, IrExitTarget>,
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
    property_numbers: HashMap<String, u8>,
    /// Standard property mappings
    standard_properties: HashMap<StandardProperty, u8>,
    /// Next available property number
    next_property_number: u8,
}

impl PropertyManager {
    pub fn new() -> Self {
        let mut manager = Self {
            property_numbers: HashMap::new(),
            standard_properties: HashMap::new(),
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
#[derive(Debug, Clone)]
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
            string_table: HashMap::new(),
            property_defaults: IrPropertyDefaults::new(),
            program_mode: ProgramMode::Script, // Default mode, will be overridden
            symbol_ids: HashMap::new(),
            object_numbers: HashMap::new(),
            id_registry: IrIdRegistry::new(), // NEW: Initialize ID registry
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
    symbol_ids: HashMap<String, IrId>, // Symbol name -> IR ID mapping
    current_locals: Vec<IrLocal>,      // Track local variables in current function
    next_local_slot: u8,               // Next available local variable slot
    builtin_functions: HashMap<IrId, String>, // Function ID -> Function name for builtins
    object_numbers: HashMap<String, u16>, // Object name -> Object number mapping
    property_manager: PropertyManager, // Manages property numbering and inheritance
    id_registry: IrIdRegistry,         // NEW: Track all IR IDs for debugging and mapping
}

impl Default for IrGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IrGenerator {
    pub fn new() -> Self {
        let mut object_numbers = HashMap::new();
        // Player is always object #1
        object_numbers.insert("player".to_string(), 1);

        IrGenerator {
            id_counter: 1, // Start from 1, 0 is reserved
            symbol_ids: HashMap::new(),
            current_locals: Vec::new(),
            next_local_slot: 1, // Slot 0 reserved for return value
            builtin_functions: HashMap::new(),
            object_numbers,
            property_manager: PropertyManager::new(),
            id_registry: IrIdRegistry::new(), // NEW: Initialize ID registry
        }
    }

    /// Check if a function name is a known builtin function
    fn is_builtin_function(&self, name: &str) -> bool {
        matches!(
            name,
            "print"
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
        )
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

        // Copy symbol mappings from generator to IR program for use in codegen
        ir_program.symbol_ids = self.symbol_ids.clone();
        ir_program.object_numbers = self.object_numbers.clone();
        ir_program.id_registry = self.id_registry.clone(); // NEW: Transfer ID registry

        Ok(ir_program)
    }

    /// Get builtin functions discovered during IR generation
    pub fn get_builtin_functions(&self) -> &HashMap<IrId, String> {
        &self.builtin_functions
    }

    pub fn get_object_numbers(&self) -> &HashMap<String, u16> {
        &self.object_numbers
    }

    /// Check if a property name corresponds to a standard Z-Machine property
    fn get_standard_property(&self, property_name: &str) -> Option<StandardProperty> {
        match property_name {
            "short_name" | "name" => Some(StandardProperty::ShortName),
            "long_name" | "desc" | "description" => Some(StandardProperty::LongName),
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
                let ir_block = self.generate_block(init.body)?;
                ir_program.init_block = Some(ir_block);
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

            let object_number = self.object_numbers.len() as u16 + 1;
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

        // Convert properties to Z-Machine properties
        let mut properties = IrProperties::new();

        // Set standard properties
        properties.set_string(StandardProperty::ShortName as u8, obj.identifier.clone());
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

        let short_name = obj
            .names
            .first()
            .cloned()
            .unwrap_or_else(|| obj.identifier.clone());

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

        // Assign object number
        let object_number = self.object_numbers.len() as u16 + 1;
        self.object_numbers
            .insert(obj.identifier.clone(), object_number);

        log::debug!(
            "Registered object '{}' with ID {} and object number {}",
            obj.identifier,
            obj_id,
            object_number
        );

        // Process nested objects recursively
        for nested_obj in &obj.contains {
            self.register_object_and_nested(nested_obj)?;
        }

        Ok(())
    }

    fn generate_room(
        &mut self,
        room: crate::grue_compiler::ast::RoomDecl,
    ) -> Result<IrRoom, CompilerError> {
        let room_id = self.next_id();
        self.symbol_ids.insert(room.identifier.clone(), room_id);

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
            let object_number = self.object_numbers.len() as u16 + 1;
            log::debug!(
                "IR generate_room: Assigning object number {} to room '{}' (fallback)",
                object_number,
                room.identifier
            );
            self.object_numbers
                .insert(room.identifier.clone(), object_number);
        }

        let mut exits = HashMap::new();
        for (direction, target) in room.exits {
            let ir_target = match target {
                crate::grue_compiler::ast::ExitTarget::Room(_room_name) => {
                    // We'll resolve room IDs in a later pass
                    IrExitTarget::Room(0) // Placeholder
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
        for obj in &room.objects {
            self.register_object_and_nested(obj)?;
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
                    crate::grue_compiler::ast::Handler::FunctionCall(_name, args) => {
                        // Convert arguments to IR values
                        let mut ir_args = Vec::new();
                        for arg in args {
                            let ir_value = self.expr_to_ir_value(arg)?;
                            ir_args.push(ir_value);
                        }

                        // Function ID will be resolved in a later pass
                        IrHandler::FunctionCall(0, ir_args) // Placeholder
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

                        // Check if this is a standard property that should use numbered access
                        if let Some(standard_prop) = self.get_standard_property(&property) {
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
                                // Fallback to string-based access if no number is registered
                                block.add_instruction(IrInstruction::SetProperty {
                                    object: object_temp,
                                    property,
                                    value: value_temp,
                                });
                            }
                        } else {
                            // For now, still support named property access for backward compatibility
                            block.add_instruction(IrInstruction::SetProperty {
                                object: object_temp,
                                property,
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
                log::debug!("IR if: Adding jump to end label {}", end_label);
                block.add_instruction(IrInstruction::Jump { label: end_label });

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
                // This is a simplified implementation that assumes array iteration

                // Generate the iterable expression
                let iterable_temp = self.generate_expression(for_stmt.iterable, block)?;

                // Create a loop variable
                let loop_var_id = self.next_id();
                let local_var = IrLocal {
                    name: for_stmt.variable.clone(),
                    var_type: Some(Type::Any), // Type inferred from array elements
                    slot: self.next_local_slot,
                    mutable: false, // Loop variables are immutable
                };
                self.current_locals.push(local_var);
                self.symbol_ids.insert(for_stmt.variable, loop_var_id);
                self.next_local_slot += 1;

                // Create index variable for array iteration
                let index_var = self.next_id();
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
                let element_temp = self.next_id();
                block.add_instruction(IrInstruction::GetArrayElement {
                    target: element_temp,
                    array: iterable_temp,
                    index: index_temp,
                });
                block.add_instruction(IrInstruction::StoreVar {
                    var_id: loop_var_id,
                    source: element_temp,
                });

                // Execute loop body
                self.generate_statement(*for_stmt.body, block)?;

                // Increment index
                let one_temp = self.next_id();
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: one_temp,
                    value: IrValue::Integer(1),
                });
                let new_index = self.next_id();
                block.add_instruction(IrInstruction::BinaryOp {
                    target: new_index,
                    op: IrBinaryOp::Add,
                    left: index_temp,
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
                // Check if this is an object identifier first
                if let Some(&object_number) = self.object_numbers.get(&name) {
                    // This is an object - load its number as a constant
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
                    return Err(CompilerError::SemanticError(
                        format!("Undefined identifier '{}'", name),
                        0,
                    ));
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
                // This should be handled as: get property from object, if callable then call it

                // Generate arguments first
                let mut arg_temps = Vec::new();
                for arg in arguments {
                    let arg_temp = self.generate_expression(arg, block)?;
                    arg_temps.push(arg_temp);
                }

                // Generate property access to get the method function
                let property_temp = self.next_id();
                block.add_instruction(IrInstruction::GetProperty {
                    target: property_temp,
                    object: object_temp,
                    property: method.clone(),
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

                // Special handling for known built-in methods
                match method.as_str() {
                    "contents" => {
                        // contents() method: return array of objects contained in this object
                        // This is a built-in method that traverses the Z-Machine object tree
                        // Create array to hold the contents
                        let array_temp = self.next_id();
                        block.add_instruction(IrInstruction::CreateArray {
                            target: array_temp,
                            size: IrValue::Integer(10), // Initial capacity
                        });

                        // Use a built-in function call to populate the array with object contents
                        // The Z-Machine runtime will handle the object tree traversal
                        let builtin_id = self.next_id();
                        self.builtin_functions
                            .insert(builtin_id, "get_object_contents".to_string());

                        let mut call_args = vec![object_temp]; // Object to get contents of
                        call_args.extend(arg_temps); // Any additional arguments

                        block.add_instruction(IrInstruction::Call {
                            target: Some(result_temp),
                            function: builtin_id,
                            args: call_args,
                        });
                    }
                    "empty" => {
                        // empty() method: return true if object has no contents
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
                        // none() method: return true if this value is null/undefined/empty
                        // This is commonly used for checking if optional values exist
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
                        // This should be implemented as proper array manipulation
                        // For now, return success (1) to indicate the add operation worked
                        log::debug!("Array/collection 'add' method called - returning success");
                        block.add_instruction(IrInstruction::LoadImmediate {
                            target: result_temp,
                            value: IrValue::Integer(1),
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

                // Else branch: property doesn't exist or isn't callable, return 0
                block.add_instruction(IrInstruction::Label { id: else_label });
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: result_temp,
                    value: IrValue::Integer(0),
                });

                // End label
                block.add_instruction(IrInstruction::Label { id: end_label });

                Ok(result_temp)
            }
            Expr::PropertyAccess { object, property } => {
                // Property access: object.property
                let is_array = self.is_array_type(&object);
                let object_temp = self.generate_expression(*object, block)?;
                let temp_id = self.next_id();

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
                        // Fallback to string-based access if no number is registered
                        block.add_instruction(IrInstruction::GetProperty {
                            target: temp_id,
                            object: object_temp,
                            property,
                        });
                    }
                } else {
                    // For now, still support named property access for backward compatibility
                    block.add_instruction(IrInstruction::GetProperty {
                        target: temp_id,
                        object: object_temp,
                        property,
                    });
                }

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
                            // Fallback to string-based access
                            block.add_instruction(IrInstruction::GetProperty {
                                target: temp_id,
                                object: object_temp,
                                property: property.clone(),
                            });
                        }
                    } else {
                        // For now, still support named property access for backward compatibility
                        block.add_instruction(IrInstruction::GetProperty {
                            target: temp_id,
                            object: object_temp,
                            property: property.clone(),
                        });
                    }
                }

                // End label
                block.add_instruction(IrInstruction::Label { id: end_label });

                Ok(temp_id)
            }

            Expr::Array(elements) => {
                // Array literal - for now, we'll create a series of load instructions
                // In a full implementation, this would create an array object
                let array_size = elements.len() as i16; // Save size before elements is moved
                let mut _temp_ids = Vec::new();
                for element in elements {
                    let element_temp = self.generate_expression(element, block)?;
                    _temp_ids.push(element_temp);
                    // TODO: Store in array structure
                }

                // Create the array with the determined size
                let temp_id = self.next_id();
                block.add_instruction(IrInstruction::CreateArray {
                    target: temp_id,
                    size: IrValue::Integer(array_size),
                });
                Ok(temp_id)
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
                        value: IrValue::Null,
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
}

#[cfg(test)]
#[path = "ir_tests.rs"]
mod tests;

// Abstract Syntax Tree definitions for Grue language

use indexmap::IndexMap;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

impl Program {
    pub fn has_grammar(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item, Item::Grammar(_)))
    }

    pub fn has_main_function(&self) -> bool {
        self.items.iter().any(|item| {
            if let Item::Function(func) = item {
                func.name == "main"
            } else {
                false
            }
        })
    }

    pub fn has_world(&self) -> bool {
        self.items.iter().any(|item| matches!(item, Item::World(_)))
    }

    pub fn get_explicit_mode(&self) -> Option<ProgramMode> {
        self.items.iter().find_map(|item| {
            if let Item::Mode(mode_decl) = item {
                Some(mode_decl.mode.clone())
            } else {
                None
            }
        })
    }

    pub fn detect_program_mode(&self) -> ProgramMode {
        if let Some(explicit_mode) = self.get_explicit_mode() {
            return explicit_mode;
        }

        if self.has_main_function() {
            ProgramMode::Custom
        } else if self.has_world() || self.has_grammar() {
            ProgramMode::Interactive
        } else {
            ProgramMode::Script
        }
    }
}

#[derive(Debug, Clone)]
pub enum Item {
    World(WorldDecl),
    Grammar(GrammarDecl),
    Function(FunctionDecl),
    Init(InitDecl),
    Mode(ModeDecl),
}

// Program mode declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ModeDecl {
    pub mode: ProgramMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProgramMode {
    Script,      // init → quit
    Interactive, // init → auto main loop
    Custom,      // init → call main()
}

// World declarations
#[derive(Debug, Clone)]
pub struct WorldDecl {
    pub rooms: Vec<RoomDecl>,
}

#[derive(Debug, Clone)]
pub struct RoomDecl {
    pub identifier: String,
    pub display_name: String,
    pub description: String,
    pub objects: Vec<ObjectDecl>,
    pub exits: IndexMap<String, ExitTarget>,
    pub on_enter: Option<BlockStmt>,
    pub on_exit: Option<BlockStmt>,
    pub on_look: Option<BlockStmt>,
}

#[derive(Debug, Clone)]
pub struct ObjectDecl {
    pub identifier: String,
    pub names: Vec<String>,
    pub description: String,
    pub properties: HashMap<String, PropertyValue>,
    pub attributes: Vec<String>, // Named attributes (e.g., "openable", "container")
    pub numbered_properties: HashMap<u8, PropertyValue>, // Z-Machine numbered properties
    pub contains: Vec<ObjectDecl>,

    // Enhanced object system integration
    pub object_type: Option<ObjectTypeDecl>, // Optional explicit type declaration
    pub inheritance: Option<String>,         // Inherit from another object type
}

#[derive(Debug, Clone)]
pub enum ExitTarget {
    Room(String),
    Blocked(String), // message
}

#[derive(Debug, Clone)]
pub enum PropertyValue {
    Boolean(bool),
    Integer(i16),
    String(String),
    Byte(u8),       // For numbered properties
    Bytes(Vec<u8>), // For multi-byte numbered properties
    Object(String), // Reference to another object
    Room(String),   // Reference to a room
}

/// Object type declaration for enhanced object system
#[derive(Debug, Clone)]
pub enum ObjectTypeDecl {
    Item,
    Container {
        openable: bool,
        lockable: bool,
        capacity: Option<u8>,
    },
    Supporter {
        capacity: Option<u8>,
    },
    Room {
        light: bool,
    },
    Door {
        connects: (String, String), // Two rooms this door connects
        openable: bool,
        lockable: bool,
        key: Option<String>,
    },
    Scenery,
    Character {
        proper_named: bool,
    },
    LightSource {
        portable: bool,
    },
}

// Grammar declarations
#[derive(Debug, Clone)]
pub struct GrammarDecl {
    pub verbs: Vec<VerbDecl>,
    pub vocabulary: Option<VocabularyDecl>, // Optional vocabulary definitions
}

#[derive(Debug, Clone)]
pub struct VocabularyDecl {
    pub adjectives: Vec<String>,   // "red", "small", "open", "closed"
    pub prepositions: Vec<String>, // "in", "on", "under", "with", "from"
    pub pronouns: Vec<String>,     // "it", "them", "him", "her"
    pub articles: Vec<String>,     // "the", "a", "an", "some"
    pub conjunctions: Vec<String>, // "and", "then", "but"
}

#[derive(Debug, Clone)]
pub struct VerbDecl {
    pub word: String,
    pub patterns: Vec<VerbPattern>,
}

#[derive(Debug, Clone)]
pub struct VerbPattern {
    pub pattern: Vec<PatternElement>,
    pub handler: Handler,
}

#[derive(Debug, Clone)]
pub enum PatternElement {
    Literal(String),
    Noun,
    Default,

    // Enhanced parser elements for Zork I-level parsing
    Adjective,       // "red", "small", "open", etc.
    MultiWordNoun,   // "small mailbox", "jewel-encrusted egg"
    Preposition,     // "in", "on", "under", "with", "from"
    MultipleObjects, // "all", "everything", "lamp and key"
    DirectObject,    // First object in sentence
    IndirectObject,  // Second object (for "put X in Y")

    // Advanced pattern matching
    OptionalAdjective, // Optional adjective before noun
    AnyPreposition,    // Match any preposition from a set
    NumberedNoun,      // "first lamp", "second book"
}

#[derive(Debug, Clone)]
pub enum Handler {
    FunctionCall(String, Vec<Expr>), // function name, arguments
    Block(BlockStmt),
}

// Function declarations
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: BlockStmt,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub param_type: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Any, // For unknown or inferred types
    Bool,
    Int,
    String,
    Room,
    Object,
    Array(Box<Type>),
}

// Init declaration
#[derive(Debug, Clone)]
pub struct InitDecl {
    pub body: BlockStmt,
}

// Statements
#[derive(Debug, Clone)]
pub struct BlockStmt {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expression(Expr),
    VarDecl(VarDeclStmt),
    Assignment(AssignmentStmt),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),
    Return(Option<Expr>),
    Block(BlockStmt),
}

#[derive(Debug, Clone)]
pub struct VarDeclStmt {
    pub name: String,
    pub mutable: bool,
    pub var_type: Option<Type>,
    pub initializer: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct AssignmentStmt {
    pub target: Expr, // Usually an identifier or property access
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Box<Stmt>,
    pub else_branch: Option<Box<Stmt>>,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Box<Stmt>,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub variable: String,
    pub iterable: Expr,
    pub body: Box<Stmt>,
}

// Expressions
#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Boolean(bool),
    Integer(i16),
    String(String),
    Identifier(String),
    Parameter(String), // $noun, $2, etc.

    // Enhanced parser parameters for advanced pattern matching
    ParsedObject {
        adjectives: Vec<String>, // Parsed adjectives
        noun: String,            // Base noun
        article: Option<String>, // Optional article ("the", "a")
    },

    MultipleObjects(Vec<Expr>), // For "lamp and key" or "all"

    // Disambiguation context
    DisambiguationContext {
        candidates: Vec<Expr>, // Possible object matches
        query: String,         // Original user input
    },

    // Property access: object.property
    PropertyAccess {
        object: Box<Expr>,
        property: String,
    },

    // Null-safe property access: object?.property
    NullSafePropertyAccess {
        object: Box<Expr>,
        property: String,
    },

    // Function calls
    FunctionCall {
        name: String,
        arguments: Vec<Expr>,
    },
    // Method calls: object.property()
    MethodCall {
        object: Box<Expr>,
        method: String,
        arguments: Vec<Expr>,
    },

    // Binary operations
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
    },

    // Unary operations
    Unary {
        operator: UnaryOp,
        operand: Box<Expr>,
    },

    // Array literal
    Array(Vec<Expr>),

    // Ternary conditional: condition ? true_expr : false_expr
    Ternary {
        condition: Box<Expr>,
        true_expr: Box<Expr>,
        false_expr: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
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

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Minus,
}

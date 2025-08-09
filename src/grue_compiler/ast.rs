// Abstract Syntax Tree definitions for Grue language

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    World(WorldDecl),
    Grammar(GrammarDecl),
    Function(FunctionDecl),
    Init(InitDecl),
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
    pub exits: HashMap<String, ExitTarget>,
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
    pub contains: Vec<ObjectDecl>,
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
}

// Grammar declarations
#[derive(Debug, Clone)]
pub struct GrammarDecl {
    pub verbs: Vec<VerbDecl>,
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

    // Property access: object.property
    PropertyAccess {
        object: Box<Expr>,
        property: String,
    },

    // Function calls
    FunctionCall {
        name: String,
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

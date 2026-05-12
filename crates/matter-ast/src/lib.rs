/// Abstract Syntax Tree for Matter language
/// Representa a estrutura sintática do código Matter

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

/// Type annotation for gradual typing
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Simple(String),                                     // int, string, bool, float
    Nullable(Box<TypeAnnotation>),                      // int?
    NonNullable(Box<TypeAnnotation>),                   // string!
    List(Box<TypeAnnotation>),                          // [int]
    Map(Box<TypeAnnotation>, Box<TypeAnnotation>),      // map<string, int>
    Union(Vec<TypeAnnotation>),                         // int | string
    Function(Vec<TypeAnnotation>, Box<TypeAnnotation>), // fn(int, int) -> int
    Generic(String, Vec<TypeAnnotation>),               // List<T>
}

/// Function parameter with optional type
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_annotation: Option<TypeAnnotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        name: String,
        type_annotation: Option<TypeAnnotation>,
        value: Expression,
    },
    Set {
        name: String,
        value: Expression,
    },
    SetIndex {
        target: Expression,
        index: Expression,
        value: Expression,
    },
    SetField {
        target: String,
        field: String,
        value: Expression,
    },
    Print(Expression),
    FunctionDef {
        name: String,
        params: Vec<Param>,
        return_type: Option<TypeAnnotation>,
        body: Vec<Statement>,
        effects: Option<Vec<String>>, // Sprint 27.3: Effect declarations
    },
    StructDef {
        name: String,
        fields: Vec<(String, String)>,
    },
    Import {
        path: String,
    },
    OnEvent {
        event: String,
        body: Vec<Statement>,
    },
    Spawn {
        event: String,
    },
    If {
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    For {
        item: String,
        iterable: Expression,
        body: Vec<Statement>,
    },
    Loop {
        body: Vec<Statement>,
    },
    Break,
    Continue,
    Return(Expression),
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
    Identifier(String),
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    BackendCall {
        backend: String,
        method: String,
        args: Vec<Expression>,
    },
    // Sprint 4: Data Model
    List(Vec<Expression>),
    Map(Vec<(String, Expression)>),
    StructLiteral {
        type_name: String,
        fields: Vec<(String, Expression)>,
    },
    Field {
        target: Box<Expression>,
        field: String,
    },
    Index {
        target: Box<Expression>,
        index: Box<Expression>,
    },
    MethodCall {
        target: Box<Expression>,
        method: String,
        args: Vec<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
}

impl Program {
    pub fn new(statements: Vec<Statement>) -> Self {
        Self { statements }
    }
}

impl Param {
    pub fn new(name: String) -> Self {
        Self {
            name,
            type_annotation: None,
        }
    }

    pub fn with_type(name: String, type_annotation: TypeAnnotation) -> Self {
        Self {
            name,
            type_annotation: Some(type_annotation),
        }
    }
}

/// Abstract Syntax Tree for Matter language
/// Representa a estrutura sintática do código Matter

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        name: String,
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
        params: Vec<String>,
        body: Vec<Statement>,
    },
    StructDef {
        name: String,
        fields: Vec<(String, String)>,
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
    Bool(bool),
    String(String),
    Unit,
    Identifier(String),
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
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
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
}

impl Program {
    pub fn new(statements: Vec<Statement>) -> Self {
        Self { statements }
    }
}

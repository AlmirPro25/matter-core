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
    ImportFrom {
        path: String,
        names: Vec<ImportName>,
    },
    ImportAs {
        path: String,
        alias: String,
    },
    Export {
        names: Vec<String>,
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
    Match {
        subject: Expression,
        arms: Vec<MatchArm>,
    },
    Expression(Expression),
}

/// A single arm in a match statement
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Expression,
    pub body: Vec<Statement>,
}

/// Name in an import statement: `name` or `name as alias`
#[derive(Debug, Clone, PartialEq)]
pub struct ImportName {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Null,
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
    Lambda {
        params: Vec<Param>,
        body: Vec<Statement>,
    },
    OkExpr(Box<Expression>),
    ErrExpr(Box<Expression>),
    SomeExpr(Box<Expression>),
    NoneExpr,
    TryPropagate(Box<Expression>),
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

/// Surface integrity tests (commit 991dcf4):
/// Parser and bytecode already reference these variants. Incremental builds can hide
/// missing variants if only dirty crates recompile; these unit tests force `matter-ast`
/// to construct every required surface so a clean checkout fails loudly if they regress.
#[cfg(test)]
mod surface_integrity_991dcf4 {
    use super::*;

    #[test]
    fn statement_import_export_variants_constructible() {
        let _ = Statement::Import {
            path: "mod.matter".into(),
        };
        let _ = Statement::ImportFrom {
            path: "mod.matter".into(),
            names: vec![ImportName {
                name: "foo".into(),
                alias: None,
            }],
        };
        let _ = Statement::ImportAs {
            path: "mod.matter".into(),
            alias: "m".into(),
        };
        let _ = Statement::Export {
            names: vec!["foo".into()],
        };
    }

    #[test]
    fn expression_lambda_result_option_try_variants_constructible() {
        let body = vec![Statement::Return(Expression::Int(1))];
        let _ = Expression::Lambda {
            params: vec![Param::new("x".into())],
            body: body.clone(),
        };
        let _ = Expression::OkExpr(Box::new(Expression::Int(1)));
        let _ = Expression::ErrExpr(Box::new(Expression::String("e".into())));
        let _ = Expression::SomeExpr(Box::new(Expression::Bool(true)));
        let _ = Expression::NoneExpr;
        let _ = Expression::TryPropagate(Box::new(Expression::Identifier("r".into())));
    }

    #[test]
    fn statement_and_expression_enums_remain_nonempty() {
        // Touch both enums so renames/deletions of the module surface fail compile/tests.
        assert!(matches!(
            Statement::Break,
            Statement::Break | Statement::Continue
        ));
        assert!(matches!(Expression::Unit, Expression::Unit | Expression::Null));
    }
}

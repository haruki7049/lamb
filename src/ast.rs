// AST definitions for the language.
// Design goals reflected here (from the design discussion):
// - Haskell-like surface syntax (fun/let/match, algebraic data types)
// - No runtime GC: the semantic/codegen stages will later attach
//   ownership/region information to `Expr`, but the AST itself stays
//   evaluation-strategy agnostic.

pub type Span = chumsky::span::SimpleSpan;
pub type Spanned<T> = (T, Span);

#[derive(Clone, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Gt,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    // `_`
    Wildcard,
    // Binds a value to a name, e.g. `x`
    Var(String),
    // Integer literal pattern, e.g. `0`
    Int(i64),
    // Constructor pattern, e.g. `Node(x, xs)` or `Empty`
    Ctor(String, Vec<Pattern>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    // Reserved for the error-recovery combinators we'll wire up next
    // (`recover_with` / `nested_delimiters`, as seen in chumsky's own
    // examples) so a syntax error doesn't abort the whole parse.
    #[allow(dead_code)]
    Error,
    Int(i64),
    Var(String),
    // `let name = value in body`
    Let {
        name: String,
        value: Box<Spanned<Expr>>,
        body: Box<Spanned<Expr>>,
    },
    // `if cond then a else b`
    If {
        cond: Box<Spanned<Expr>>,
        then_branch: Box<Spanned<Expr>>,
        else_branch: Box<Spanned<Expr>>,
    },
    Binary(Box<Spanned<Expr>>, BinOp, Box<Spanned<Expr>>),
    // Function call or data constructor application: `f(a, b)`
    Call(Box<Spanned<Expr>>, Vec<Spanned<Expr>>),
    // `match scrutinee | pat -> expr | pat -> expr ...`
    Match {
        scrutinee: Box<Spanned<Expr>>,
        arms: Vec<(Pattern, Spanned<Expr>)>,
    },
}

// A single constructor inside a `type` declaration, e.g. `Node(a, List a)`
#[derive(Clone, Debug, PartialEq)]
pub struct CtorDecl {
    pub name: String,
    // We don't type-check field types yet, just keep their names for now.
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub ctors: Vec<CtorDecl>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Spanned<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Type(TypeDecl),
    Fun(FunDecl),
}

pub type Program = Vec<Item>;

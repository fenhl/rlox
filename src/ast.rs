pub(crate) enum Stmt {
    Var(String, Option<Expr>),
    Expr(Expr),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    Print(Expr),
    Block(Vec<Stmt>),
    //TODO others
}

pub(crate) enum Expr {
    Assign(Option<Box<Expr>>, String, Box<Expr>),
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    True,
    False,
    Nil,
    Number(f64),
    String(String),
    Variable(String),
    //TODO others
}

pub(crate) enum BinaryOp {
    Or,
    And,
    NotEqual,
    Equal,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Sub,
    Add,
    Div,
    Mul,
}

pub(crate) enum UnaryOp {
    Not,
    Neg,
}

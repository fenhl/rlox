pub(crate) enum Stmt {
    Var(String, Option<Expr>),
    Expr(Expr),
    Print(Expr),
    //TODO others
}

pub(crate) enum Expr {
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    True,
    False,
    Nil,
    Number(f64),
    Variable(String),
    //TODO others
}

pub(crate) enum BinaryOp {
    NotEqual,
    Equal,
}

pub(crate) enum UnaryOp {
    Not,
    Neg,
}

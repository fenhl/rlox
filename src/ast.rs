pub(crate) enum Stmt {
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

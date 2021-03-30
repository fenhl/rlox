pub(crate) enum Stmt {
    Expr(Expr),
    Print(Expr),
    //TODO others
}

pub(crate) enum Expr {
    Unary(UnaryOp, Box<Expr>),
    True,
    False,
    Nil,
    //TODO others
}

pub(crate) enum UnaryOp {
    Not,
    Neg,
}

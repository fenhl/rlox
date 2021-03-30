pub(crate) enum Stmt {
    Expr(Expr),
    Print(Expr),
    //TODO others
}

pub(crate) enum Expr {
    Nil,
    //TODO others
}

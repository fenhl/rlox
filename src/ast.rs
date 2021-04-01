pub(crate) enum Stmt {
    //TODO class
    //TODO fun
    Var(String, Option<Expr>),
    Expr(Expr),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    Print(Expr),
    //TODO return
    While(Expr, Box<Stmt>),
    Block(Vec<Stmt>),
}

pub(crate) enum Expr {
    Assign(Option<Box<Expr>>, String, Box<Expr>),
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    //TODO call
    //TODO property
    True,
    False,
    Nil,
    //TODO this
    Number(f64),
    String(String),
    Variable(String),
    //TODO super
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

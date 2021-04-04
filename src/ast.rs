pub(crate) enum Stmt {
    //TODO class
    Fun {
        name: String,
        name_line: u32,
        params: Vec<(u32, String)>,
        body: Vec<Stmt>,
        last_line: u32,
    },
    Var {
        name: String,
        name_line: u32,
        init: Option<Expr>,
        last_line: u32,
    },
    Expr {
        expr: Expr,
        last_line: u32,
    },
    If {
        cond: Expr,
        right_paren_line: u32,
        then: Box<Stmt>,
        else_: Option<Box<Stmt>>,
    },
    Print {
        expr: Expr,
        last_line: u32,
    },
    Return {
        keyword_line: u32,
        expr: Option<Expr>,
        last_line: u32,
    },
    While {
        cond: Expr,
        right_paren_line: u32,
        body: Box<Stmt>,
    },
    Block {
        stmts: Vec<Stmt>,
        last_line: u32,
    },
}

impl Stmt {
    pub(crate) fn last_line(&self) -> u32 {
        match self {
            Stmt::Fun { last_line, .. } | Stmt::Var { last_line, .. } | Stmt::Expr { last_line, .. } | Stmt::Print { last_line, .. } | Stmt::Return { last_line, .. } | Stmt::Block { last_line, .. } => *last_line,
            Stmt::If { then: inner, else_: None, .. } | Stmt::If { else_: Some(inner), .. } | Stmt::While { body: inner, .. } => inner.last_line(),
        }
    }
}

pub(crate) enum Expr {
    Assign {
        rcpt: Option<Box<Expr>>,
        name: String,
        name_line: u32,
        value: Box<Expr>,
    },
    Binary {
        lhs: Box<Expr>,
        op: BinaryOp,
        rhs: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        inner: Box<Expr>,
    },
    Call {
        rcpt: Box<Expr>,
        args: Vec<Expr>,
        last_line: u32,
    },
    //TODO property
    True {
        line: u32,
    },
    False {
        line: u32,
    },
    Nil {
        line: u32,
    },
    //TODO this
    Number {
        value: f64,
        line: u32,
    },
    String {
        value: String,
        last_line: u32,
    },
    Variable {
        name: String,
        line: u32,
    },
    //TODO super
}

impl Expr {
    pub(crate) fn last_line(&self) -> u32 {
        match self {
            Expr::True { line } | Expr::False { line } | Expr::Nil { line } | Expr::Number { line, .. } | Expr::Variable { line, .. } => *line,
            Expr::Call { last_line, .. } | Expr::String { last_line, .. } => *last_line,
            Expr::Assign { value: inner, .. } | Expr::Binary { rhs: inner, .. } | Expr::Unary { inner, .. } => inner.last_line(),
        }
    }
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

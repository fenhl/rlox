use {
    std::convert::{
        TryFrom as _,
        TryInto as _,
    },
    gc::Gc,
    crate::{
        ast::*,
        error::{
            Error,
            Result,
        },
        value::{
            FunctionInner,
            Value,
        },
        vm::OpCode,
    },
};

enum FunctionType {
    Function,
    Initializer,
    Method,
    Script,
}

struct Local {
    name: String,
    depth: Option<usize>,
    is_captured: bool,
}

#[must_use]
struct Jump(usize);

struct Compiler {
    function: FunctionInner,
    fn_type: FunctionType,
    locals: Vec<Local>,
    scope_depth: usize,
}

impl Compiler {
    fn new(fn_type: FunctionType) -> Compiler {
        Compiler {
            function: FunctionInner::default(),
            locals: vec![Local {
                name: String::default(), //TODO use this for methods and initializers
                depth: Some(0),
                is_captured: false,
            }],
            scope_depth: if let FunctionType::Script = fn_type { 0 } else { 1 },
            fn_type,
        }
    }

    fn compile_stmt(&mut self, stmt: Stmt) -> Result {
        match stmt {
            Stmt::Var { name, name_line, init, last_line } => {
                let global = self.declare_variable(name_line, name, false)?;
                if let Some(init) = init {
                    self.compile_expr(init)?;
                } else {
                    self.emit(name_line, OpCode::Nil);
                }
                self.define_variable(last_line, global);
            }
            Stmt::Expr { expr, last_line } => {
                self.compile_expr(expr)?;
                self.emit(last_line, OpCode::Pop);
            }
            Stmt::Fun { name, name_line, params, body, last_line } => {
                let arity = params.len().try_into().map_err(|_| Error::Compile {
                    msg: format!("Can't have more than 255 parameters."),
                    line: params[255].0,
                })?;
                let global = self.declare_variable(name_line, name.clone(), true)?; //TODO wrap in Gc to avoid the clone?
                let mut compiler = Compiler::new(FunctionType::Function);
                compiler.function.arity = arity;
                compiler.function.name = Some(Gc::new(name));
                for (line, param) in params {
                    compiler.declare_variable(line, param, true)?;
                }
                for stmt in body {
                    compiler.compile_stmt(stmt)?;
                }
                self.emit_constant(last_line, OpCode::Closure, Value::new(compiler.finalize(last_line).wrap()))?;
                self.define_variable(last_line, global);
            }
            Stmt::If { cond, right_paren_line, then, else_: Some(else_), .. } => {
                self.compile_expr(cond)?;
                let then_jump = self.emit_jump(right_paren_line, OpCode::JumpIfFalsePop);
                let then_last_line = then.last_line();
                self.compile_stmt(*then)?;
                let else_jump = self.emit_jump(then_last_line, OpCode::Jump);
                self.patch_jump(then_last_line, then_jump)?;
                let else_last_line = else_.last_line();
                self.compile_stmt(*else_)?;
                self.patch_jump(else_last_line, else_jump)?;
            }
            Stmt::If { cond, right_paren_line, then, else_: None, .. } => {
                self.compile_expr(cond)?;
                let then_jump = self.emit_jump(right_paren_line, OpCode::JumpIfFalsePop);
                let then_last_line = then.last_line();
                self.compile_stmt(*then)?;
                self.patch_jump(then_last_line, then_jump)?;
            }
            Stmt::Print { expr, last_line } => {
                self.compile_expr(expr)?;
                self.emit(last_line, OpCode::Print);
            }
            Stmt::Return { keyword_line, expr, last_line } => {
                if let FunctionType::Script = self.fn_type {
                    return Err(Error::Compile {
                        msg: format!("Can't return from top-level code."),
                        line: keyword_line,
                    })
                }
                if let Some(expr) = expr {
                    if let FunctionType::Initializer = self.fn_type {
                        return Err(Error::Compile {
                            msg: format!("Can't return a vale from an initializer."),
                            line: keyword_line,
                        })
                    }
                    self.compile_expr(expr)?;
                    self.emit(last_line, OpCode::Return);
                } else {
                    self.emit_return(last_line);
                }
            }
            Stmt::While { cond, right_paren_line, body, .. } => {
                let loop_start = self.function.chunk.len();
                self.compile_expr(cond)?;
                let exit_jump = self.emit_jump(right_paren_line, OpCode::JumpIfFalsePop);
                let body_last_line = body.last_line();
                self.compile_stmt(*body)?;
                self.emit_loop(body_last_line, loop_start)?;
                self.patch_jump(body_last_line, exit_jump)?;
            }
            Stmt::Block { stmts, last_line } => {
                self.begin_scope();
                for stmt in stmts {
                    self.compile_stmt(stmt)?;
                }
                self.end_scope(last_line);
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: Expr) -> Result {
        match expr {
            Expr::Assign { rcpt: Some(_), .. } => unimplemented!(), //TODO
            Expr::Assign { rcpt: None, name, name_line, value } => {
                let (arg, op) = if let Some(offset) = self.resolve_local(name_line, &name)? {
                    (offset, OpCode::SetLocal)
                } else { //TODO upvalues
                    (self.make_constant(name_line, Value::new(name))?, OpCode::SetGlobal)
                };
                let value_last_line = value.last_line();
                self.compile_expr(*value)?;
                self.emit_with_arg(value_last_line, op, arg);
            }
            Expr::Binary { lhs, op: BinaryOp::Or, rhs } => {
                let lhs_last_line = lhs.last_line();
                self.compile_expr(*lhs)?;
                let jump = self.emit_jump(lhs_last_line, OpCode::JumpIfTruePeek);
                self.emit(lhs_last_line, OpCode::Pop);
                let rhs_last_line = rhs.last_line();
                self.compile_expr(*rhs)?;
                self.patch_jump(rhs_last_line, jump)?;
            }
            Expr::Binary { lhs, op: BinaryOp::And, rhs } => {
                let lhs_last_line = lhs.last_line();
                self.compile_expr(*lhs)?;
                let jump = self.emit_jump(lhs_last_line, OpCode::JumpIfFalsePeek);
                self.emit(lhs_last_line, OpCode::Pop);
                let rhs_last_line = rhs.last_line();
                self.compile_expr(*rhs)?;
                self.patch_jump(rhs_last_line, jump)?;
            }
            Expr::Binary { lhs, op, rhs } => {
                self.compile_expr(*lhs)?;
                let rhs_last_line = rhs.last_line();
                self.compile_expr(*rhs)?;
                match op {
                    BinaryOp::Or => unreachable!(), // handled above
                    BinaryOp::And => unreachable!(), // handled above
                    BinaryOp::NotEqual => {
                        self.emit(rhs_last_line, OpCode::Equal);
                        self.emit(rhs_last_line, OpCode::Not);
                    }
                    BinaryOp::Equal => self.emit(rhs_last_line, OpCode::Equal),
                    BinaryOp::Greater => self.emit(rhs_last_line, OpCode::Greater),
                    BinaryOp::GreaterEqual => self.emit(rhs_last_line, OpCode::GreaterEqual),
                    BinaryOp::Less => self.emit(rhs_last_line, OpCode::Less),
                    BinaryOp::LessEqual => self.emit(rhs_last_line, OpCode::LessEqual),
                    BinaryOp::Sub => self.emit(rhs_last_line, OpCode::Sub),
                    BinaryOp::Add => self.emit(rhs_last_line, OpCode::Add),
                    BinaryOp::Div => self.emit(rhs_last_line, OpCode::Div),
                    BinaryOp::Mul => self.emit(rhs_last_line, OpCode::Mul),
                }
            }
            Expr::Unary { op, inner } => {
                let last_line = inner.last_line();
                self.compile_expr(*inner)?;
                self.emit(last_line, match op {
                    UnaryOp::Not => OpCode::Not,
                    UnaryOp::Neg => OpCode::Neg,
                });
            }
            Expr::Call { rcpt, args, last_line } => {
                let arg_count = args.len().try_into().map_err(|_| Error::Compile {
                    msg: format!("Can't have more than 255 arguments."),
                    line: args[255].last_line(),
                })?;
                self.compile_expr(*rcpt)?;
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_with_arg(last_line, OpCode::Call, arg_count);
            }
            Expr::True { line } => self.emit(line, OpCode::True),
            Expr::False { line } => self.emit(line, OpCode::False),
            Expr::Nil { line } => self.emit(line, OpCode::Nil),
            Expr::Number { value, line } => self.emit_constant(line, OpCode::Constant, Value::new(value))?,
            Expr::String { value, last_line } => self.emit_constant(last_line, OpCode::Constant, Value::new(value))?,
            Expr::Variable { name, line } => {
                let (arg, op) = if let Some(offset) = self.resolve_local(line, &name)? {
                    (offset, OpCode::GetLocal)
                } else { //TODO upvalues
                    (self.make_constant(line, Value::new(name))?, OpCode::GetGlobal)
                };
                self.emit_with_arg(line, op, arg);
            }
        }
        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self, line: u32) {
        self.scope_depth -= 1;
        while self.locals.last().map_or(false, |local| local.depth.expect("undefined local at end of scope") > self.scope_depth) {
            self.emit(line, OpCode::Pop);
            self.locals.pop();
        }
        //TODO close captured upvalues instead
    }

    fn declare_variable(&mut self, name_line: u32, name: String, initialized: bool) -> Result<u8> {
        if self.scope_depth > 0 {
            for local in self.locals.iter().rev() {
                if local.depth.map_or(false, |depth| depth < self.scope_depth) { break }
                if local.name == name {
                    return Err(Error::Compile {
                        msg: format!("Already variable with this name in this scope."),
                        line: name_line,
                    })
                }
            }
            if self.locals.len() > u8::MAX.into() {
                return Err(Error::Compile {
                    msg: format!("Too many local variables in function."),
                    line: name_line,
                })
            }
            self.locals.push(Local {
                name,
                depth: initialized.then(|| self.scope_depth),
                is_captured: false,
            });
            return Ok(0)
        }
        //TODO intern variable name?
        self.make_constant(name_line, Value::new(name))
    }

    fn define_variable(&mut self, line: u32, global: u8) {
        if self.scope_depth > 0 {
            self.locals.last_mut().expect("no local to mark as initialized").depth = Some(self.scope_depth);
        } else {
            self.emit_with_arg(line, OpCode::DefineGlobal, global);
        }
    }

    fn resolve_local(&self, line: u32, name: &str) -> Result<Option<u8>> {
        Ok(if let Some((idx, local)) = self.locals.iter().enumerate().rfind(|(_, local)| local.name == name) {
            if local.depth.is_none() {
                return Err(Error::Compile {
                    msg: format!("Can't read local variable in its own initializer."),
                    line,
                })
            }
            Some(idx as u8)
        } else {
            None
        })
    }

    fn emit(&mut self, line: u32, opcode: OpCode) {
        self.function.add_code(line, opcode as u8);
    }

    fn emit_with_arg(&mut self, line: u32, opcode: OpCode, arg: u8) {
        self.emit(line, opcode);
        self.function.add_code(line, arg);
    }

    fn emit_constant(&mut self, line: u32, opcode: OpCode, value: Gc<Value>) -> Result {
        let const_idx = self.make_constant(line, value)?;
        self.emit_with_arg(line, opcode, const_idx);
        Ok(())
    }

    fn make_constant(&mut self, line: u32, value: Gc<Value>) -> Result<u8> {
        self.function.add_constant(value).try_into().map_err(|_| Error::Compile {
            msg: format!("Too many constants in one chunk."),
            line,
        })
    }

    fn emit_jump(&mut self, line: u32, opcode: OpCode) -> Jump {
        self.emit(line, opcode);
        self.function.add_code(line, 0);
        self.function.add_code(line, 0);
        Jump(self.function.chunk.len() - 2)
    }

    fn patch_jump(&mut self, line: u32, Jump(from_idx): Jump) -> Result {
        let offset = u16::try_from(self.function.chunk.len() - from_idx - 2).map_err(|_| Error::Compile {
            msg: format!("Too much code to jump over."),
            line,
        })?;
        self.function.chunk.splice(from_idx..from_idx + 2, std::array::IntoIter::new(offset.to_le_bytes()));
        Ok(())
    }

    fn emit_loop(&mut self, line: u32, loop_start: usize) -> Result {
        self.emit(line, OpCode::Loop);
        let offset = u16::try_from(self.function.chunk.len() - loop_start + 2).map_err(|_| Error::Compile {
            msg: format!("Loop body too large."),
            line,
        })?;
        let [b1, b2] = offset.to_le_bytes();
        self.function.add_code(line, b1);
        self.function.add_code(line, b2);
        Ok(())
    }

    fn emit_return(&mut self, line: u32) {
        if let FunctionType::Initializer = self.fn_type {
            self.emit_with_arg(line, OpCode::GetLocal, 0);
        } else {
            self.emit(line, OpCode::Nil);
        }
        self.emit(line, OpCode::Return);
    }

    fn finalize(mut self, line: u32) -> FunctionInner {
        self.emit_return(line);
        self.function
    }
}

pub(crate) fn compile(body: Vec<Stmt>) -> Result<FunctionInner> {
    let last_line = body.last().map_or(0, |stmt| stmt.last_line());
    let mut compiler = Compiler::new(FunctionType::Script);
    for stmt in body {
        compiler.compile_stmt(stmt)?;
    }
    Ok(compiler.finalize(last_line))
}

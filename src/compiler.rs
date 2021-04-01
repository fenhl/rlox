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
            Stmt::Var(name, init) => {
                let global = self.declare_variable(name, false)?;
                if let Some(init) = init {
                    self.compile_expr(init)?;
                } else {
                    self.emit(OpCode::Nil);
                }
                self.define_variable(global);
            }
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                self.emit(OpCode::Pop);
            }
            Stmt::Fun(name, params, body) => {
                if params.len() > 255 { return Err(Error::Compile(format!("Can't have more than 255 parameters."))) }
                let global = self.declare_variable(name.clone(), true)?; //TODO wrap in Gc to avoid the clone?
                let mut compiler = Compiler::new(FunctionType::Function);
                compiler.function.name = Some(Gc::new(name));
                for param in params {
                    compiler.declare_variable(param, true)?;
                }
                for stmt in body {
                    compiler.compile_stmt(stmt)?;
                }
                self.emit_constant(OpCode::Closure, Value::new(compiler.finalize().wrap()))?;
                self.define_variable(global);
            }
            Stmt::If(cond, then, Some(else_)) => {
                self.compile_expr(cond)?;
                let then_jump = self.emit_jump(OpCode::JumpIfFalsePop);
                self.compile_stmt(*then)?;
                let else_jump = self.emit_jump(OpCode::Jump);
                self.patch_jump(then_jump)?;
                self.compile_stmt(*else_)?;
                self.patch_jump(else_jump)?;
            }
            Stmt::If(cond, then, None) => {
                self.compile_expr(cond)?;
                let then_jump = self.emit_jump(OpCode::JumpIfFalsePop);
                self.compile_stmt(*then)?;
                self.patch_jump(then_jump)?;
            }
            Stmt::Print(expr) => {
                self.compile_expr(expr)?;
                self.emit(OpCode::Print);
            }
            Stmt::While(cond, body) => {
                let loop_start = self.function.chunk.len();
                self.compile_expr(cond)?;
                let exit_jump = self.emit_jump(OpCode::JumpIfFalsePop);
                self.compile_stmt(*body)?;
                self.emit_loop(loop_start)?;
                self.patch_jump(exit_jump)?;
            }
            Stmt::Block(stmts) => {
                self.begin_scope();
                for stmt in stmts {
                    self.compile_stmt(stmt)?;
                }
                self.end_scope();
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: Expr) -> Result {
        match expr {
            Expr::Assign(Some(_), _, _) => unimplemented!(), //TODO
            Expr::Assign(None, name, value) => {
                let (arg, op) = if let Some(offset) = self.resolve_local(&name)? {
                    (offset, OpCode::SetLocal)
                } else { //TODO upvalues
                    (self.make_constant(Value::new(name))?, OpCode::SetGlobal)
                };
                self.compile_expr(*value)?;
                self.emit_with_arg(op, arg);
            }
            Expr::Binary(lhs, BinaryOp::Or, rhs) => {
                self.compile_expr(*lhs)?;
                let jump = self.emit_jump(OpCode::JumpIfTruePeek);
                self.emit(OpCode::Pop);
                self.compile_expr(*rhs)?;
                self.patch_jump(jump)?;
            }
            Expr::Binary(lhs, BinaryOp::And, rhs) => {
                self.compile_expr(*lhs)?;
                let jump = self.emit_jump(OpCode::JumpIfFalsePeek);
                self.emit(OpCode::Pop);
                self.compile_expr(*rhs)?;
                self.patch_jump(jump)?;
            }
            Expr::Binary(lhs, op, rhs) => {
                self.compile_expr(*lhs)?;
                self.compile_expr(*rhs)?;
                match op {
                    BinaryOp::Or => unreachable!(), // handled above
                    BinaryOp::And => unreachable!(), // handled above
                    BinaryOp::NotEqual => {
                        self.emit(OpCode::Equal);
                        self.emit(OpCode::Not);
                    }
                    BinaryOp::Equal => self.emit(OpCode::Equal),
                    BinaryOp::Greater => self.emit(OpCode::Greater),
                    BinaryOp::GreaterEqual => self.emit(OpCode::GreaterEqual),
                    BinaryOp::Less => self.emit(OpCode::Less),
                    BinaryOp::LessEqual => self.emit(OpCode::LessEqual),
                    BinaryOp::Sub => self.emit(OpCode::Sub),
                    BinaryOp::Add => self.emit(OpCode::Add),
                    BinaryOp::Div => self.emit(OpCode::Div),
                    BinaryOp::Mul => self.emit(OpCode::Mul),
                }
            }
            Expr::Unary(op, inner) => {
                self.compile_expr(*inner)?;
                self.emit(match op {
                    UnaryOp::Not => OpCode::Not,
                    UnaryOp::Neg => OpCode::Neg,
                });
            }
            Expr::Call(rcpt, args) => {
                let arg_count = args.len().try_into().map_err(|_| Error::Compile(format!("Can't have more than 255 arguments.")))?;
                self.compile_expr(*rcpt)?;
                for arg in args {
                    self.compile_expr(arg)?;
                }
                self.emit_with_arg(OpCode::Call, arg_count);
            }
            Expr::True => self.emit(OpCode::True),
            Expr::False => self.emit(OpCode::False),
            Expr::Nil => self.emit(OpCode::Nil),
            Expr::Number(n) => self.emit_constant(OpCode::Constant, Value::new(n))?,
            Expr::String(s) => self.emit_constant(OpCode::Constant, Value::new(s))?,
            Expr::Variable(name) => {
                let (arg, op) = if let Some(offset) = self.resolve_local(&name)? {
                    (offset, OpCode::GetLocal)
                } else { //TODO upvalues
                    (self.make_constant(Value::new(name))?, OpCode::GetGlobal)
                };
                self.emit_with_arg(op, arg);
            }
        }
        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;
        while self.locals.last().map_or(false, |local| local.depth.expect("undefined local at end of scope") > self.scope_depth) {
            self.emit(OpCode::Pop);
            self.locals.pop();
        }
        //TODO close captured upvalues instead
    }

    fn declare_variable(&mut self, name: String, initialized: bool) -> Result<u8> {
        if self.scope_depth > 0 {
            for local in self.locals.iter().rev() {
                if local.depth.map_or(false, |depth| depth < self.scope_depth) { break }
                if local.name == name { return Err(Error::Compile(format!("Already variable with this name in this scope."))) }
            }
            if self.locals.len() > u8::MAX.into() { return Err(Error::Compile(format!("Too many local variables in function."))) }
            self.locals.push(Local {
                name,
                depth: initialized.then(|| self.scope_depth),
                is_captured: false,
            });
            return Ok(0)
        }
        //TODO intern variable name?
        self.make_constant(Value::new(name))
    }

    fn define_variable(&mut self, global: u8) {
        if self.scope_depth > 0 {
            self.locals.last_mut().expect("no local to mark as initialized").depth = Some(self.scope_depth);
        } else {
            self.emit_with_arg(OpCode::DefineGlobal, global);
        }
    }

    fn resolve_local(&self, name: &str) -> Result<Option<u8>> {
        Ok(if let Some((idx, local)) = self.locals.iter().enumerate().rfind(|(_, local)| local.name == name) {
            if local.depth.is_none() { return Err(Error::Compile(format!("Can't read local variable in its own initializer."))) }
            Some(idx as u8)
        } else {
            None
        })
    }

    fn emit(&mut self, opcode: OpCode) {
        self.function.chunk.push(opcode as u8);
    }

    fn emit_with_arg(&mut self, opcode: OpCode, arg: u8) {
        self.emit(opcode);
        self.function.chunk.push(arg);
    }

    fn emit_constant(&mut self, opcode: OpCode, value: Gc<Value>) -> Result {
        let const_idx = self.make_constant(value)?;
        self.emit_with_arg(opcode, const_idx);
        Ok(())
    }

    fn make_constant(&mut self, value: Gc<Value>) -> Result<u8> {
        self.function.add_constant(value).try_into().map_err(|_| Error::Compile(format!("Too many constants in one chunk.")))
    }

    fn emit_jump(&mut self, opcode: OpCode) -> Jump {
        self.emit(opcode);
        self.function.chunk.push(0);
        self.function.chunk.push(0);
        Jump(self.function.chunk.len() - 2)
    }

    fn patch_jump(&mut self, Jump(from_idx): Jump) -> Result {
        let offset = u16::try_from(self.function.chunk.len() - from_idx - 2).map_err(|_| Error::Compile(format!("Too much code to jump over.")))?;
        self.function.chunk.splice(from_idx..from_idx + 2, std::array::IntoIter::new(offset.to_le_bytes()));
        Ok(())
    }

    fn emit_loop(&mut self, loop_start: usize) -> Result {
        self.emit(OpCode::Loop);
        let offset = u16::try_from(self.function.chunk.len() - loop_start + 2).map_err(|_| Error::Compile(format!("Loop body too large.")))?;
        self.function.chunk.extend(std::array::IntoIter::new(offset.to_le_bytes()));
        Ok(())
    }

    fn emit_return(&mut self) {
        if let FunctionType::Initializer = self.fn_type {
            self.emit_with_arg(OpCode::GetLocal, 0);
        } else {
            self.emit(OpCode::Nil);
        }
        self.emit(OpCode::Return);
    }

    fn finalize(mut self) -> FunctionInner {
        self.emit_return();
        self.function
    }
}

pub(crate) fn compile(body: Vec<Stmt>) -> Result<FunctionInner> {
    let mut compiler = Compiler::new(FunctionType::Script);
    for stmt in body {
        compiler.compile_stmt(stmt)?;
    }
    Ok(compiler.finalize())
}

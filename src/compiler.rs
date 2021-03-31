use {
    std::convert::TryInto as _,
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

struct Compiler {
    function: FunctionInner,
    fn_type: FunctionType,
}

impl Compiler {
    fn new() -> Compiler {
        Compiler {
            function: FunctionInner::default(),
            fn_type: FunctionType::Script,
        }
    }

    fn compile_expr(&mut self, expr: Expr) -> Result {
        match expr {
            Expr::Binary(lhs, op, rhs) => {
                self.compile_expr(*lhs)?;
                self.compile_expr(*rhs)?;
                match op {
                    BinaryOp::NotEqual => {
                        self.emit(OpCode::Equal);
                        self.emit(OpCode::Not);
                    }
                    BinaryOp::Equal => self.emit(OpCode::Equal),
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
            Expr::True => self.emit(OpCode::True),
            Expr::False => self.emit(OpCode::False),
            Expr::Nil => self.emit(OpCode::Nil),
            Expr::Number(n) => self.emit_constant(Value::new(n))?,
            Expr::Variable(name) => {
                //TODO locals and upvalues
                let arg = self.make_constant(Value::new(name))?;
                self.emit_with_arg(OpCode::GetGlobal, arg);
            }
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: Stmt) -> Result {
        match stmt {
            Stmt::Var(name, init) => {
                let global = self.declare_variable(name)?;
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
            Stmt::Print(expr) => {
                self.compile_expr(expr)?;
                self.emit(OpCode::Print);
            }
        }
        Ok(())
    }

    fn declare_variable(&mut self, name: String) -> Result<u8> {
        //TODO handle declarations inside scopes
        //TODO intern variable name?
        self.make_constant(Value::new(name))
    }

    fn define_variable(&mut self, global: u8) {
        //TODO handle definitions inside scopes
        self.emit_with_arg(OpCode::DefineGlobal, global);
    }

    fn emit(&mut self, opcode: OpCode) {
        self.function.chunk.push(opcode as u8);
    }

    fn emit_with_arg(&mut self, opcode: OpCode, arg: u8) {
        self.emit(opcode);
        self.function.chunk.push(arg);
    }

    fn emit_constant(&mut self, value: Gc<Value>) -> Result {
        let const_idx = self.make_constant(value)?;
        self.emit_with_arg(OpCode::Constant, const_idx);
        Ok(())
    }

    fn make_constant(&mut self, value: Gc<Value>) -> Result<u8> {
        self.function.add_constant(value).try_into().map_err(|_| Error::Compile(format!("Too many constants in one chunk.")))
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
    let mut compiler = Compiler::new();
    for stmt in body {
        compiler.compile_stmt(stmt)?;
    }
    compiler.emit_return();
    Ok(compiler.finalize())
}

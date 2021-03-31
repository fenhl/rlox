use {
    std::{
        collections::HashMap,
        mem,
    },
    gc::Gc,
    crate::{
        error::{
            Error,
            Result,
        },
        value::{
            Closure,
            FunctionInner,
            Value,
        },
    },
};

#[repr(u8)]
pub(crate) enum OpCode {
    Add,
    Constant,
    DefineGlobal,
    Div,
    Equal,
    False,
    GetGlobal,
    GetLocal,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Mul,
    Neg,
    Nil,
    Not,
    Pop,
    Print,
    Return,
    Sub,
    True,
}

struct CallFrame {
    closure: Gc<Closure>,
    ip: usize,
    slots_start: usize,
}

pub(crate) struct Vm {
    frames: Vec<CallFrame>,
    stack: Vec<Gc<Value>>,
    globals: HashMap<Gc<String>, Gc<Value>>,
}

impl Vm {
    pub(crate) fn new() -> Vm {
        Vm {
            frames: Vec::default(),
            stack: Vec::default(),
            globals: HashMap::default(), //TODO define `clock` native
        }
    }

    pub(crate) fn interpret(&mut self, function: FunctionInner) -> Result {
        let closure = Closure::new(function.wrap());
        self.push(Value::new(closure.clone()));
        self.call(closure, 0)?;
        self.run()?;
        Ok(())
    }

    fn run(&mut self) -> Result {
        macro_rules! frame {
            () => {
                &mut self.frames.last_mut().expect("call frame stack empty")
            };
        }

        macro_rules! read_byte {
            () => {{
                let frame = frame!();
                let byte = frame.closure.function.borrow().chunk[frame.ip];
                frame.ip += 1;
                byte
            }};
        }

        macro_rules! read_constant {
            () => {{
                let const_idx = usize::from(read_byte!());
                &frame!().closure.function.borrow().constants[const_idx]
            }};
        }

        loop {
            let instruction = unsafe { mem::transmute::<u8, OpCode>(read_byte!()) };
            match instruction {
                OpCode::Add => {
                    let rhs = self.pop();
                    let lhs = self.pop();
                    self.push(match (&*lhs, &*rhs) {
                        (Value::String(lhs), Value::String(rhs)) => Value::new(format!("{}{}", lhs, rhs)),
                        (Value::Number(lhs), Value::Number(rhs)) => Value::new(lhs + rhs),
                        (_, _) => return Err(Error::Runtime(format!("Operands must be two numbers or two strings."))),
                    });
                }
                OpCode::Constant => {
                    let value = read_constant!().clone();
                    self.push(value);
                }
                OpCode::DefineGlobal => {
                    let name = read_constant!().as_string().expect("global name was not a string");
                    let value = self.pop();
                    self.globals.insert(name, value);
                }
                OpCode::Div => {
                    let rhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    let lhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    self.push(Value::new(lhs / rhs));
                }
                OpCode::Equal => {
                    let rhs = self.pop();
                    let lhs = self.pop();
                    self.push(Value::new(lhs == rhs));
                }
                OpCode::False => self.push(Value::new(false)),
                OpCode::GetGlobal => {
                    let name = read_constant!().as_string().expect("global name was not a string");
                    let value = self.globals.get(&name).ok_or_else(|| Error::Runtime(format!("Undefined variable '{}'.", name)))?.clone();
                    self.push(value);
                }
                OpCode::GetLocal => {
                    let slot = read_byte!();
                    let local = self.stack[frame!().slots_start + usize::from(slot)].clone();
                    self.push(local);
                }
                OpCode::Greater => {
                    let rhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    let lhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    self.push(Value::new(lhs > rhs));
                }
                OpCode::GreaterEqual => {
                    let rhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    let lhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    self.push(Value::new(lhs >= rhs));
                }
                OpCode::Less => {
                    let rhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    let lhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    self.push(Value::new(lhs < rhs));
                }
                OpCode::LessEqual => {
                    let rhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    let lhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    self.push(Value::new(lhs <= rhs));
                }
                OpCode::Mul => {
                    let rhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    let lhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    self.push(Value::new(lhs * rhs));
                }
                OpCode::Neg => {
                    let n = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operand must be a number.")))?;
                    self.push(Value::new(-n));
                }
                OpCode::Nil => self.push(Value::nil()),
                OpCode::Not => {
                    let operand = self.pop();
                    self.push(Value::new(!operand.as_bool()));
                }
                OpCode::Pop => { let _ = self.pop(); }
                OpCode::Print => println!("{}", self.pop()),
                OpCode::Return => {
                    let result = self.pop();
                    //TODO close upvalues
                    let _ = self.frames.pop();
                    if self.frames.is_empty() {
                        let _ = self.pop();
                        return Ok(())
                    }
                    self.stack.truncate(frame!().slots_start);
                    self.push(result);
                }
                OpCode::Sub => {
                    let rhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    let lhs = self.pop().as_number().ok_or_else(|| Error::Runtime(format!("Operands must be numbers.")))?;
                    self.push(Value::new(lhs - rhs));
                }
                OpCode::True => self.push(Value::new(true)),
            }
        }
    }

    fn call(&mut self, closure: Gc<Closure>, arg_count: u8) -> Result {
        let arity = closure.function.borrow().arity;
        if arg_count != arity {
            return Err(Error::Runtime(format!("Expected {} arguments but got {}.", arity, arg_count)))
        }
        //TODO hardcode stack limit?
        self.frames.push(CallFrame {
            closure,
            ip: 0,
            slots_start: self.stack.len() - usize::from(arg_count) - 1,
        });
        Ok(())
    }

    fn pop(&mut self) -> Gc<Value> {
        self.stack.pop().expect("tried to pop from an empty VM stack")
    }

    fn push(&mut self, value: Gc<Value>) {
        self.stack.push(value);
    }
}

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

macro_rules! error {
    ($vm:expr, $($arg:tt)*) => {{
        let vm = $vm;
        return Err(Error::Runtime {
            msg: format!($($arg)*),
            call_stack: vm.frames.clone(),
        })
    }};
}

macro_rules! expect {
    ($vm:expr, $option:expr, $($arg:tt)*) => {{
        $option.ok_or_else(|| Error::Runtime {
            msg: format!($($arg)*),
            call_stack: $vm.frames.clone(),
        })?
    }};
}

const FRAMES_MAX: usize = 64;

#[repr(u8)]
#[derive(Debug)]
pub(crate) enum OpCode {
    Add,
    Call,
    Closure,
    Constant,
    DefineGlobal,
    Div,
    Equal,
    False,
    GetGlobal,
    GetLocal,
    Greater,
    GreaterEqual,
    Jump,
    JumpIfFalsePeek,
    JumpIfFalsePop,
    JumpIfTruePeek,
    Less,
    LessEqual,
    Loop,
    Mul,
    Neg,
    Nil,
    Not,
    Pop,
    Print,
    Return,
    SetGlobal,
    SetLocal,
    Sub,
    True,
}

impl OpCode {
    pub(crate) fn disassemble(chunk: &mut &[u8], constants: &[Gc<Value>]) {
        use OpCode::*;

        let instruction = unsafe { mem::transmute::<u8, OpCode>(chunk[0]) };
        *chunk = &chunk[1..];
        match instruction {
            Add | Div | Equal | False | Greater | GreaterEqual | Less | LessEqual | Mul | Neg | Nil | Not | Pop | Print | Return | Sub | True => println!("{:?}", instruction),
            Call | GetLocal | SetLocal => {
                let arg = chunk[0];
                *chunk = &chunk[1..];
                println!("{:?} 0x{:02x}", instruction, arg);
            }
            Closure | Constant | DefineGlobal | GetGlobal | SetGlobal => {
                let arg = chunk[0];
                *chunk = &chunk[1..];
                let constant = &constants[usize::from(arg)];
                println!("{:?} 0x{:02x} ({})", instruction, arg, constant);
            }
            Jump | JumpIfFalsePeek | JumpIfFalsePop | JumpIfTruePeek | Loop => {
                let offset = u16::from_le_bytes([chunk[0], chunk[1]]);
                *chunk = &chunk[2..];
                println!("{:?} 0x{:04x}", instruction, offset);
            }
        }
    }
}

#[derive(Clone)]
pub struct CallFrame {
    pub(crate) closure: Gc<Closure>,
    pub(crate) ip: usize,
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
            globals: crate::native::all(),
        }
    }

    pub(crate) fn interpret(&mut self, function: FunctionInner) -> Result {
        let closure = Closure::new(function.wrap());
        self.push(Value::new(closure.clone()));
        self.call(closure, 0)?;
        self.run()
    }

    fn run(&mut self) -> Result {
        macro_rules! frame {
            () => {
                &mut self.frames.last_mut().expect("call frame stack empty")
            };
        }

        macro_rules! read_u8 {
            () => {{
                let frame = frame!();
                let byte = frame.closure.function.borrow().chunk[frame.ip];
                frame.ip += 1;
                byte
            }};
        }

        macro_rules! read_u16 {
            () => {
                u16::from_le_bytes([read_u8!(), read_u8!()])
            };
        }

        macro_rules! read_constant {
            () => {{
                let const_idx = usize::from(read_u8!());
                &frame!().closure.function.borrow().constants[const_idx]
            }};
        }

        loop {
            let instruction = unsafe { mem::transmute::<u8, OpCode>(read_u8!()) };
            match instruction {
                OpCode::Add => {
                    let rhs = self.pop();
                    let lhs = self.pop();
                    self.push(match (&*lhs, &*rhs) {
                        (Value::String(lhs), Value::String(rhs)) => Value::new(format!("{}{}", lhs, rhs)),
                        (Value::Number(lhs), Value::Number(rhs)) => Value::new(lhs + rhs),
                        (_, _) => error!(self, "Operands must be two numbers or two strings."),
                    });
                }
                OpCode::Call => {
                    let arg_count = read_u8!();
                    let rcpt = self.peek(arg_count.into()).clone();
                    self.call_value(rcpt, arg_count)?;
                }
                OpCode::Closure => {
                    let function = read_constant!().as_function().expect("function constant was not a function");
                    self.push(Value::new(Closure::new(function)));
                    //TODO capture upvalues
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
                    let rhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    let lhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
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
                    let value = expect!(self, self.globals.get(&name), "Undefined variable '{}'.", name).clone();
                    self.push(value);
                }
                OpCode::GetLocal => {
                    let slot = read_u8!();
                    let local = self.stack[frame!().slots_start + usize::from(slot)].clone();
                    self.push(local);
                }
                OpCode::Greater => {
                    let rhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    let lhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    self.push(Value::new(lhs > rhs));
                }
                OpCode::GreaterEqual => {
                    let rhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    let lhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    self.push(Value::new(lhs >= rhs));
                }
                OpCode::Jump => {
                    let offset = read_u16!();
                    frame!().ip += usize::from(offset);
                }
                OpCode::JumpIfFalsePeek => {
                    let offset = read_u16!();
                    if !self.peek(0).as_bool() { frame!().ip += usize::from(offset) }
                }
                OpCode::JumpIfFalsePop => {
                    let offset = read_u16!();
                    if !self.pop().as_bool() { frame!().ip += usize::from(offset) }
                }
                OpCode::JumpIfTruePeek => {
                    let offset = read_u16!();
                    if self.peek(0).as_bool() { frame!().ip += usize::from(offset) }
                }
                OpCode::Less => {
                    let rhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    let lhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    self.push(Value::new(lhs < rhs));
                }
                OpCode::LessEqual => {
                    let rhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    let lhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    self.push(Value::new(lhs <= rhs));
                }
                OpCode::Loop => {
                    let offset = read_u16!();
                    frame!().ip -= usize::from(offset);
                }
                OpCode::Mul => {
                    let rhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    let lhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    self.push(Value::new(lhs * rhs));
                }
                OpCode::Neg => {
                    let n = expect!(self, self.pop().as_number(), "Operand must be a number.");
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
                    let popped_frame = self.frames.pop().expect("tried to return from empty call stack");
                    if self.frames.is_empty() {
                        let _ = self.pop();
                        return Ok(())
                    }
                    self.stack.truncate(popped_frame.slots_start);
                    self.push(result);
                }
                OpCode::SetGlobal => {
                    let name = read_constant!().as_string().expect("global name was not a string");
                    let value = self.peek(0).clone();
                    if self.globals.insert(name.clone(), value).is_none() {
                        self.globals.remove(&name);
                        error!(self, "Undefined variable '{}'.", name)
                    }
                }
                OpCode::SetLocal => {
                    let slot = read_u8!();
                    self.stack[frame!().slots_start + usize::from(slot)] = self.peek(0).clone();
                }
                OpCode::Sub => {
                    let rhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    let lhs = expect!(self, self.pop().as_number(), "Operands must be numbers.");
                    self.push(Value::new(lhs - rhs));
                }
                OpCode::True => self.push(Value::new(true)),
            }
        }
    }

    fn call_value(&mut self, value: Gc<Value>, arg_count: u8) -> Result {
        match *value {
            Value::Closure(ref closure) => self.call(closure.clone(), arg_count),
            Value::NativeFn(crate::value::NativeFn { inner }) => {
                let result = inner(&self.stack[self.stack.len() - usize::from(arg_count)..self.stack.len()]);
                self.stack.truncate(self.stack.len() - usize::from(arg_count) - 1);
                self.push(result);
                Ok(())
            }
            //TODO bound methods, classes
            _ => error!(self, "Can only call functions and classes."),
        }
    }

    fn call(&mut self, closure: Gc<Closure>, arg_count: u8) -> Result {
        let arity = closure.function.borrow().arity;
        if arg_count != arity { error!(self, "Expected {} arguments but got {}.", arity, arg_count) }
        if self.frames.len() == FRAMES_MAX { error!(self, "Stack overflow.") }
        self.frames.push(CallFrame {
            closure,
            ip: 0,
            slots_start: self.stack.len() - usize::from(arg_count) - 1,
        });
        Ok(())
    }

    fn push(&mut self, value: Gc<Value>) {
        self.stack.push(value);
    }

    fn peek(&self, offset: usize) -> &Gc<Value> {
        &self.stack[self.stack.len() - 1 - offset]
    }

    fn pop(&mut self) -> Gc<Value> {
        self.stack.pop().expect("tried to pop from an empty VM stack")
    }
}

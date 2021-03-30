use {
    std::mem,
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
    False,
    GetLocal,
    Neg,
    Nil,
    Not,
    Pop,
    Print,
    Return,
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
}

impl Vm {
    pub(crate) fn new() -> Vm {
        Vm {
            frames: Vec::default(),
            stack: Vec::default(),
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

        loop {
            let instruction = unsafe { mem::transmute::<u8, OpCode>(read_byte!()) };
            match instruction {
                OpCode::False => self.push(Value::new(false)),
                OpCode::GetLocal => {
                    let slot = read_byte!();
                    let local = self.stack[frame!().slots_start + usize::from(slot)].clone();
                    self.push(local);
                }
                OpCode::Neg => return Err(Error::Runtime(format!("Operand must be a number."))), //TODO handle numbers
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
